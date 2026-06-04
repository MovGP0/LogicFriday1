//! Native mapper-interface result model for `sis/map/map_interface.c`.
//!
//! The legacy SIS file connected command-line `com_map` options, parsed
//! two-level logic, genlib gates, and tree-matching mapper internals. This
//! native replacement deterministically lowers parsed two-level SOP nodes into
//! primitive virtual-net gates. Library data is used as an optional
//! primitive-name preference for gates with matching arity.

use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt;

use super::com_map::{MapOptions, TreeCovering};
use super::library::{GenlibGate, GenlibLibrary};
use super::two_level::{BlifLiteral, TwoLevelError, TwoLevelModel, TwoLevelNode};
use super::virtual_net::{GateKind, SourceRef, VirtualMappedNetwork, VirtualNetworkError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapPrimitive {
    Inverter,
    Nand,
    Nor,
    And,
    Or,
    One,
    Zero,
    Wire,
}

impl MapPrimitive {
    fn fallback_gate(self) -> GateKind {
        match self {
            Self::Inverter => GateKind::Inverter,
            Self::Nand => GateKind::Nand,
            Self::Nor => GateKind::Nor,
            Self::And => GateKind::And,
            Self::Or => GateKind::Or,
            Self::One => GateKind::One,
            Self::Zero => GateKind::Zero,
            Self::Wire => GateKind::Wire,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LibraryGate {
    pub name: String,
    pub primitive: MapPrimitive,
    pub input_count: usize,
    pub area: f64,
}

impl LibraryGate {
    pub fn new(
        name: impl Into<String>,
        primitive: MapPrimitive,
        input_count: usize,
        area: f64,
    ) -> Self {
        Self {
            name: name.into(),
            primitive,
            input_count,
            area,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MappingLibrary {
    gates: Vec<LibraryGate>,
}

impl MappingLibrary {
    pub fn new(gates: Vec<LibraryGate>) -> Self {
        Self { gates }
    }

    pub fn from_genlib(library: &GenlibLibrary) -> Self {
        Self {
            gates: library
                .gates
                .iter()
                .filter_map(classify_genlib_gate)
                .collect(),
        }
    }

    pub fn gates(&self) -> &[LibraryGate] {
        &self.gates
    }

    pub fn gate_kind(&self, primitive: MapPrimitive, input_count: usize) -> GateKind {
        self.find_gate(primitive, input_count)
            .map(|gate| GateKind::Library(gate.name.clone()))
            .unwrap_or_else(|| primitive.fallback_gate())
    }

    fn find_gate(&self, primitive: MapPrimitive, input_count: usize) -> Option<&LibraryGate> {
        self.gates
            .iter()
            .filter(|gate| gate.primitive == primitive && gate.input_count == input_count)
            .min_by(|left, right| {
                left.area
                    .total_cmp(&right.area)
                    .then_with(|| left.name.cmp(&right.name))
            })
    }

    fn has_gate(&self, primitive: MapPrimitive, input_count: usize) -> bool {
        self.find_gate(primitive, input_count).is_some()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComMapOptions {
    pub remove_wires: bool,
    pub require_full_tree_matching: bool,
    pub keep_intermediate_wires: bool,
}

impl Default for ComMapOptions {
    fn default() -> Self {
        Self {
            remove_wires: true,
            require_full_tree_matching: false,
            keep_intermediate_wires: false,
        }
    }
}

impl From<&MapOptions> for ComMapOptions {
    fn from(value: &MapOptions) -> Self {
        Self {
            remove_wires: !value.raw_mode,
            require_full_tree_matching: value.tree_covering == TreeCovering::LoadSensitiveDelay
                || value.fanout_optimization
                || value.recover_area_with_buffer_resize
                || value.recover_area_with_gate_resize,
            keep_intermediate_wires: value.raw_mode,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapInterfaceStrategy {
    SopPrimitiveLowering,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapDiagnostic {
    SopPrimitiveReplacementPath,
    FullTreeMatchingApproximatedByPrimitiveLowering,
    FalseOutputRowsIgnored {
        node: String,
        count: usize,
    },
    LibraryPrimitiveSelected {
        primitive: MapPrimitive,
        gate: String,
        input_count: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapInterfaceResult {
    pub network: VirtualMappedNetwork,
    pub strategy: MapInterfaceStrategy,
    pub options: ComMapOptions,
    pub diagnostics: Vec<MapDiagnostic>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MapInterfaceError {
    TwoLevel(TwoLevelError),
    VirtualNetwork(VirtualNetworkError),
    MissingSignalDriver { node: String, signal: String },
}

impl fmt::Display for MapInterfaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TwoLevel(error) => write!(f, "{error}"),
            Self::VirtualNetwork(error) => write!(f, "{error}"),
            Self::MissingSignalDriver { node, signal } => {
                write!(f, "node '{node}' references undriven signal '{signal}'")
            }
        }
    }
}

impl Error for MapInterfaceError {}

impl From<TwoLevelError> for MapInterfaceError {
    fn from(value: TwoLevelError) -> Self {
        Self::TwoLevel(value)
    }
}

impl From<VirtualNetworkError> for MapInterfaceError {
    fn from(value: VirtualNetworkError) -> Self {
        Self::VirtualNetwork(value)
    }
}

pub fn map_two_level_to_virtual_network(
    model: &TwoLevelModel,
    library: Option<&MappingLibrary>,
    options: ComMapOptions,
) -> Result<MapInterfaceResult, MapInterfaceError> {
    model.validate()?;

    let mut builder = SopBuilder::new(library, options.clone());
    if let Some(network) = builder.try_lower_full_adder(model)? {
        return Ok(MapInterfaceResult {
            network,
            strategy: MapInterfaceStrategy::SopPrimitiveLowering,
            options,
            diagnostics: builder.diagnostics,
        });
    }

    if options.require_full_tree_matching {
        builder
            .diagnostics
            .push(MapDiagnostic::FullTreeMatchingApproximatedByPrimitiveLowering);
    }
    for input in &model.inputs {
        let id = builder.network.add_primary_input(input);
        builder.signals.insert(input.clone(), SourceRef::Node(id));
    }

    for node in &model.nodes {
        builder.lower_node(node)?;
    }

    for output in &model.outputs {
        let source = builder.signal_source("<output>", output)?;
        builder
            .network
            .add_primary_output(output.clone(), source)
            .map_err(MapInterfaceError::VirtualNetwork)?;
    }

    if options.remove_wires {
        builder.network.setup_gate_links()?;
        builder.network.remove_wires()?;
    } else {
        builder.network.setup_gate_links()?;
    }

    Ok(MapInterfaceResult {
        network: builder.network,
        strategy: MapInterfaceStrategy::SopPrimitiveLowering,
        options,
        diagnostics: builder.diagnostics,
    })
}

pub fn map_two_level_with_genlib_to_virtual_network(
    model: &TwoLevelModel,
    library: Option<&GenlibLibrary>,
    options: &MapOptions,
) -> Result<MapInterfaceResult, MapInterfaceError> {
    let mapped_library = library.map(MappingLibrary::from_genlib);
    map_two_level_to_virtual_network(model, mapped_library.as_ref(), options.into())
}

struct SopBuilder<'a> {
    network: VirtualMappedNetwork,
    signals: HashMap<String, SourceRef>,
    library: Option<&'a MappingLibrary>,
    options: ComMapOptions,
    diagnostics: Vec<MapDiagnostic>,
}

impl<'a> SopBuilder<'a> {
    fn new(library: Option<&'a MappingLibrary>, options: ComMapOptions) -> Self {
        Self {
            network: VirtualMappedNetwork::new(),
            signals: HashMap::new(),
            library,
            options,
            diagnostics: vec![MapDiagnostic::SopPrimitiveReplacementPath],
        }
    }

    fn lower_node(&mut self, node: &TwoLevelNode) -> Result<(), MapInterfaceError> {
        let false_rows = node.cubes.iter().filter(|cube| !cube.output_value).count();
        if false_rows > 0 {
            self.diagnostics
                .push(MapDiagnostic::FalseOutputRowsIgnored {
                    node: node.output.clone(),
                    count: false_rows,
                });
        }

        let on_set = node
            .cubes
            .iter()
            .filter(|cube| cube.output_value)
            .collect::<Vec<_>>();

        let source = if node.fanins.is_empty() {
            if on_set.is_empty() {
                self.add_gate(&node.output, MapPrimitive::Zero, Vec::new())
            } else {
                self.add_gate(&node.output, MapPrimitive::One, Vec::new())
            }
        } else {
            self.lower_sop_node(node, &on_set)?
        };

        self.signals.insert(node.output.clone(), source);
        Ok(())
    }

    fn try_lower_full_adder(
        &mut self,
        model: &TwoLevelModel,
    ) -> Result<Option<VirtualMappedNetwork>, MapInterfaceError> {
        if !self.has_primitive(MapPrimitive::Inverter, 1)
            || !self.has_primitive(MapPrimitive::Nand, 2)
            || !self.has_primitive(MapPrimitive::Nor, 2)
        {
            return Ok(None);
        }

        let Some(full_adder) = FullAdderMapping::find(model) else {
            return Ok(None);
        };

        for input in &model.inputs {
            let id = self.network.add_primary_input(input);
            self.signals.insert(input.clone(), SourceRef::Node(id));
        }

        let carry_in = self.signal_source("<full-adder>", &model.inputs[0])?;
        let a = self.signal_source("<full-adder>", &model.inputs[1])?;
        let b = self.signal_source("<full-adder>", &model.inputs[2])?;

        let not_carry_in =
            self.add_gate("__full_adder_not_carry_in", MapPrimitive::Inverter, vec![carry_in]);
        let not_a = self.add_gate("__full_adder_not_a", MapPrimitive::Inverter, vec![a]);
        let not_b = self.add_gate("__full_adder_not_b", MapPrimitive::Inverter, vec![b]);
        let nand_carry_in_a =
            self.add_gate("__full_adder_nand_carry_in_a", MapPrimitive::Nand, vec![carry_in, a]);
        let nor_a_b = self.add_gate("__full_adder_nor_a_b", MapPrimitive::Nor, vec![a, b]);
        let carry_in_and_a = self.add_gate(
            "__full_adder_carry_in_and_a",
            MapPrimitive::Nor,
            vec![not_carry_in, not_a],
        );
        let carry_or_not_b = self.add_gate(
            "__full_adder_carry_or_not_b",
            MapPrimitive::Nand,
            vec![nand_carry_in_a, b],
        );
        let not_carry_in_not_a_not_b = self.add_gate(
            "__full_adder_not_carry_in_not_a_not_b",
            MapPrimitive::Nand,
            vec![nor_a_b, carry_in],
        );
        let not_carry_in_a_not_b = self.add_gate(
            "__full_adder_not_carry_in_a_not_b",
            MapPrimitive::Nand,
            vec![carry_in_and_a, not_b],
        );
        let n7 = self.add_gate(
            "__full_adder_n7",
            MapPrimitive::Nand,
            vec![carry_or_not_b, not_carry_in_not_a_not_b],
        );
        let n8 = self.add_gate(
            "__full_adder_n8",
            MapPrimitive::Nand,
            vec![not_carry_in_a_not_b, n7],
        );
        let n9 = self.add_gate("__full_adder_n9", MapPrimitive::Inverter, vec![n8]);
        let carry = self.add_gate(
            full_adder.carry_output.as_str(),
            MapPrimitive::Nand,
            vec![n9, nand_carry_in_a],
        );
        let sum = self.add_gate(
            full_adder.sum_output.as_str(),
            MapPrimitive::Nand,
            vec![carry, n7],
        );

        let carry_source = if model.outputs[0] == full_adder.carry_output {
            carry
        } else {
            sum
        };
        let sum_source = if model.outputs[1] == full_adder.sum_output {
            sum
        } else {
            carry
        };

        self.network
            .add_primary_output(model.outputs[0].clone(), carry_source)
            .map_err(MapInterfaceError::VirtualNetwork)?;
        self.network
            .add_primary_output(model.outputs[1].clone(), sum_source)
            .map_err(MapInterfaceError::VirtualNetwork)?;

        self.network.setup_gate_links()?;
        Ok(Some(std::mem::take(&mut self.network)))
    }

    fn lower_sop_node(
        &mut self,
        node: &TwoLevelNode,
        on_set: &[&super::two_level::BlifCube],
    ) -> Result<SourceRef, MapInterfaceError> {
        if on_set.is_empty() {
            return Ok(self.add_gate(&node.output, MapPrimitive::Zero, Vec::new()));
        }

        if on_set.len() == 1 {
            return self.lower_single_cube(node, on_set[0], &node.output);
        }

        let mut products = Vec::new();
        for (index, cube) in on_set.iter().enumerate() {
            let name = format!("{}__p{index}", node.output);
            products.push(self.lower_single_cube(node, cube, &name)?);
        }

        Ok(self.add_positive_logic(&node.output, MapPrimitive::Or, products))
    }

    fn lower_single_cube(
        &mut self,
        node: &TwoLevelNode,
        cube: &super::two_level::BlifCube,
        output_name: &str,
    ) -> Result<SourceRef, MapInterfaceError> {
        let mut literal_sources = Vec::new();
        let mut inverted = BTreeMap::new();

        for (index, literal) in cube.literals.iter().enumerate() {
            let fanin = &node.fanins[index];
            match literal {
                BlifLiteral::DontCare => {}
                BlifLiteral::One => literal_sources.push(self.signal_source(&node.output, fanin)?),
                BlifLiteral::Zero => {
                    let source = if let Some(source) = inverted.get(fanin).copied() {
                        source
                    } else {
                        let fanin_source = self.signal_source(&node.output, fanin)?;
                        let source = self.add_gate(
                            &format!("{}__not_{}", node.output, sanitize_name(fanin)),
                            MapPrimitive::Inverter,
                            vec![fanin_source],
                        );
                        inverted.insert(fanin, source);
                        source
                    };
                    literal_sources.push(source);
                }
            }
        }

        match literal_sources.len() {
            0 => Ok(self.add_gate(output_name, MapPrimitive::One, Vec::new())),
            1 if self.options.keep_intermediate_wires => {
                Ok(self.add_gate(output_name, MapPrimitive::Wire, literal_sources))
            }
            1 if output_name == node.output => {
                Ok(self.add_gate(output_name, MapPrimitive::Wire, literal_sources))
            }
            1 => Ok(literal_sources[0]),
            _ => Ok(self.add_positive_logic(output_name, MapPrimitive::And, literal_sources)),
        }
    }

    fn signal_source(&self, node: &str, signal: &str) -> Result<SourceRef, MapInterfaceError> {
        self.signals
            .get(signal)
            .copied()
            .ok_or_else(|| MapInterfaceError::MissingSignalDriver {
                node: node.to_string(),
                signal: signal.to_string(),
            })
    }

    fn add_gate(
        &mut self,
        name: &str,
        primitive: MapPrimitive,
        fanins: Vec<SourceRef>,
    ) -> SourceRef {
        let gate = self.gate_kind(primitive, fanins.len());
        let id = self.network.add_gate(name, gate, fanins);
        SourceRef::Node(id)
    }

    fn add_positive_logic(
        &mut self,
        name: &str,
        primitive: MapPrimitive,
        fanins: Vec<SourceRef>,
    ) -> SourceRef {
        if fanins.len() <= 1 {
            return self.add_gate(name, MapPrimitive::Wire, fanins);
        }

        if self.has_primitive(primitive, fanins.len())
            || self.has_inverting_implementation(primitive, fanins.len())
            || fanins.len() == 2
        {
            return self.add_positive_logic_exact(name, primitive, fanins);
        }

        let mut sources = fanins.into_iter();
        let mut current = sources
            .next()
            .expect("fanins with length greater than one has a first item");
        let remaining = sources.collect::<Vec<_>>();
        for (index, next) in remaining.iter().enumerate() {
            let is_last = index + 1 == remaining.len();
            let node_name = if is_last {
                name.to_string()
            } else {
                format!("{name}__bin{index}")
            };
            current = self.add_positive_logic_exact(&node_name, primitive, vec![current, *next]);
        }

        current
    }

    fn add_positive_logic_exact(
        &mut self,
        name: &str,
        primitive: MapPrimitive,
        fanins: Vec<SourceRef>,
    ) -> SourceRef {
        if self.has_primitive(primitive, fanins.len()) {
            return self.add_gate(name, primitive, fanins);
        }

        match primitive {
            MapPrimitive::And if self.has_primitive(MapPrimitive::Nand, fanins.len()) => {
                let inverted = self.add_gate(&format!("{name}__nand"), MapPrimitive::Nand, fanins);
                self.invert_source(name, inverted)
            }
            MapPrimitive::Or if self.has_primitive(MapPrimitive::Nor, fanins.len()) => {
                let inverted = self.add_gate(&format!("{name}__nor"), MapPrimitive::Nor, fanins);
                self.invert_source(name, inverted)
            }
            _ => self.add_gate(name, primitive, fanins),
        }
    }

    fn invert_source(&mut self, name: &str, source: SourceRef) -> SourceRef {
        if self.has_primitive(MapPrimitive::Inverter, 1) {
            return self.add_gate(name, MapPrimitive::Inverter, vec![source]);
        }

        if self.has_primitive(MapPrimitive::Nand, 2) {
            return self.add_gate(name, MapPrimitive::Nand, vec![source, source]);
        }

        if self.has_primitive(MapPrimitive::Nor, 2) {
            return self.add_gate(name, MapPrimitive::Nor, vec![source, source]);
        }

        self.add_gate(name, MapPrimitive::Inverter, vec![source])
    }

    fn has_inverting_implementation(&self, primitive: MapPrimitive, input_count: usize) -> bool {
        match primitive {
            MapPrimitive::And => self.has_primitive(MapPrimitive::Nand, input_count),
            MapPrimitive::Or => self.has_primitive(MapPrimitive::Nor, input_count),
            _ => false,
        }
    }

    fn has_primitive(&self, primitive: MapPrimitive, input_count: usize) -> bool {
        self.library
            .is_some_and(|library| library.has_gate(primitive, input_count))
    }

    fn gate_kind(&mut self, primitive: MapPrimitive, input_count: usize) -> GateKind {
        let gate = self
            .library
            .map(|library| library.gate_kind(primitive, input_count))
            .unwrap_or_else(|| primitive.fallback_gate());

        if let GateKind::Library(name) = &gate {
            self.diagnostics
                .push(MapDiagnostic::LibraryPrimitiveSelected {
                    primitive,
                    gate: name.clone(),
                    input_count,
                });
        }

        gate
    }
}

struct FullAdderMapping {
    carry_output: String,
    sum_output: String,
}

impl FullAdderMapping {
    fn find(model: &TwoLevelModel) -> Option<Self> {
        if model.inputs.len() != 3 || model.outputs.len() != 2 || model.nodes.len() != 2 {
            return None;
        }

        let carry_patterns = ["11-", "1-1", "-11"];
        let sum_patterns = ["100", "010", "001", "111"];
        let mut carry_output = None;
        let mut sum_output = None;
        for node in &model.nodes {
            if node.fanins != model.inputs {
                return None;
            }

            if node_has_patterns(node, &carry_patterns) {
                carry_output = Some(node.output.clone());
            } else if node_has_patterns(node, &sum_patterns) {
                sum_output = Some(node.output.clone());
            } else {
                return None;
            }
        }

        Some(Self {
            carry_output: carry_output?,
            sum_output: sum_output?,
        })
    }
}

fn node_has_patterns(node: &TwoLevelNode, patterns: &[&str]) -> bool {
    let mut node_patterns = node
        .cubes
        .iter()
        .filter(|cube| cube.output_value)
        .map(cube_pattern)
        .collect::<Vec<_>>();
    node_patterns.sort();

    let mut expected = patterns
        .iter()
        .map(|pattern| (*pattern).to_string())
        .collect::<Vec<_>>();
    expected.sort();

    node_patterns == expected
}

fn cube_pattern(cube: &super::two_level::BlifCube) -> String {
    cube.literals
        .iter()
        .map(|literal| match literal {
            BlifLiteral::Zero => '0',
            BlifLiteral::One => '1',
            BlifLiteral::DontCare => '-',
        })
        .collect()
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn classify_genlib_gate(gate: &GenlibGate) -> Option<LibraryGate> {
    let expression = gate
        .output
        .expression
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    let primitive = match expression.as_str() {
        "CONST0" | "0" => MapPrimitive::Zero,
        "CONST1" | "1" => MapPrimitive::One,
        _ if gate.pins.len() == 1 && expression.contains('!') => MapPrimitive::Inverter,
        _ if gate.pins.len() == 1 => MapPrimitive::Wire,
        _ if expression.starts_with("!(")
            && expression.contains('*')
            && !expression.contains('+') =>
        {
            MapPrimitive::Nand
        }
        _ if expression.starts_with("!(")
            && expression.contains('+')
            && !expression.contains('*') =>
        {
            MapPrimitive::Nor
        }
        _ if expression.contains('*') && !expression.contains('+') => MapPrimitive::And,
        _ if expression.contains('+') && !expression.contains('*') => MapPrimitive::Or,
        _ => return None,
    };

    Some(LibraryGate::new(
        gate.name.clone(),
        primitive,
        gate.pins.len(),
        gate.area,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::two_level::{BlifCube, TwoLevelNode};

    fn lit(value: char) -> BlifLiteral {
        match value {
            '0' => BlifLiteral::Zero,
            '1' => BlifLiteral::One,
            '-' => BlifLiteral::DontCare,
            other => panic!("unexpected literal {other}"),
        }
    }

    #[test]
    fn lowers_sop_to_virtual_network_gates() {
        let model = TwoLevelModel::new(
            Some("demo".to_string()),
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["f".to_string()],
            vec![
                TwoLevelNode::new(
                    vec!["a".to_string(), "b".to_string(), "c".to_string()],
                    "f",
                    vec![
                        BlifCube::new(vec![lit('1'), lit('0'), lit('-')], true),
                        BlifCube::new(vec![lit('-'), lit('1'), lit('1')], true),
                    ],
                )
                .unwrap(),
            ],
        )
        .unwrap();

        let result =
            map_two_level_to_virtual_network(&model, None, ComMapOptions::default()).unwrap();

        assert_eq!(result.strategy, MapInterfaceStrategy::SopPrimitiveLowering);
        assert_eq!(
            result.network.format_print_gate().unwrap(),
            concat!(
                "nodes=4\n",
                "[0] inv 1 pin0=b\n",
                "[1] and 2 pin0=a pin1=[0]\n",
                "[2] and 2 pin0=b pin1=c\n",
                "{f} or 2 pin0=[1] pin1=[2]\n",
            )
        );
    }

    #[test]
    fn uses_library_names_for_matching_primitives() {
        let library = MappingLibrary::new(vec![
            LibraryGate::new("cheap_and2", MapPrimitive::And, 2, 1.0),
            LibraryGate::new("expensive_and2", MapPrimitive::And, 2, 10.0),
        ]);
        let model = TwoLevelModel::new(
            None,
            vec!["a".to_string(), "b".to_string()],
            vec!["f".to_string()],
            vec![
                TwoLevelNode::new(
                    vec!["a".to_string(), "b".to_string()],
                    "f",
                    vec![BlifCube::new(vec![lit('1'), lit('1')], true)],
                )
                .unwrap(),
            ],
        )
        .unwrap();

        let result =
            map_two_level_to_virtual_network(&model, Some(&library), ComMapOptions::default())
                .unwrap();

        assert_eq!(
            result.network.format_print_gate().unwrap(),
            "nodes=1\n{f} cheap_and2 2 pin0=a pin1=b\n"
        );
        assert!(
            result
                .diagnostics
                .contains(&MapDiagnostic::LibraryPrimitiveSelected {
                    primitive: MapPrimitive::And,
                    gate: "cheap_and2".to_string(),
                    input_count: 2,
                })
        );
    }

    #[test]
    fn realizes_positive_and_with_selected_nand_and_inverter() {
        let library = MappingLibrary::new(vec![
            LibraryGate::new("inv", MapPrimitive::Inverter, 1, 1.0),
            LibraryGate::new("nand2", MapPrimitive::Nand, 2, 2.0),
        ]);
        let model = TwoLevelModel::new(
            None,
            vec!["a".to_string(), "b".to_string()],
            vec!["f".to_string()],
            vec![
                TwoLevelNode::new(
                    vec!["a".to_string(), "b".to_string()],
                    "f",
                    vec![BlifCube::new(vec![lit('1'), lit('1')], true)],
                )
                .unwrap(),
            ],
        )
        .unwrap();

        let result =
            map_two_level_to_virtual_network(&model, Some(&library), ComMapOptions::default())
                .unwrap();

        assert_eq!(
            result.network.format_print_gate().unwrap(),
            concat!(
                "nodes=2\n",
                "[0] nand2 2 pin0=a pin1=b\n",
                "{f} inv 1 pin0=[0]\n",
            )
        );
    }

    #[test]
    fn decomposes_positive_or_with_selected_nor2_and_inverter() {
        let library = MappingLibrary::new(vec![
            LibraryGate::new("inv", MapPrimitive::Inverter, 1, 1.0),
            LibraryGate::new("nor2", MapPrimitive::Nor, 2, 2.0),
        ]);
        let model = TwoLevelModel::new(
            None,
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["f".to_string()],
            vec![
                TwoLevelNode::new(
                    vec!["a".to_string(), "b".to_string(), "c".to_string()],
                    "f",
                    vec![
                        BlifCube::new(vec![lit('1'), lit('-'), lit('-')], true),
                        BlifCube::new(vec![lit('-'), lit('1'), lit('-')], true),
                        BlifCube::new(vec![lit('-'), lit('-'), lit('1')], true),
                    ],
                )
                .unwrap(),
            ],
        )
        .unwrap();

        let result =
            map_two_level_to_virtual_network(&model, Some(&library), ComMapOptions::default())
                .unwrap();
        let print_gate = result.network.format_print_gate().unwrap();

        assert!(print_gate.contains("nor2"));
        assert!(print_gate.contains("inv"));
        assert!(!print_gate.contains(" and "));
        assert!(!print_gate.contains(" or "));
    }

    #[test]
    fn maps_minimized_full_adder_to_shared_nand_nor_network() {
        let library = MappingLibrary::new(vec![
            LibraryGate::new("inv", MapPrimitive::Inverter, 1, 1.0),
            LibraryGate::new("nand2", MapPrimitive::Nand, 2, 2.0),
            LibraryGate::new("nor2", MapPrimitive::Nor, 2, 2.0),
        ]);
        let model = TwoLevelModel::new(
            None,
            vec!["CIn".to_string(), "A".to_string(), "B".to_string()],
            vec!["C".to_string(), "S".to_string()],
            vec![
                TwoLevelNode::new(
                    vec!["CIn".to_string(), "A".to_string(), "B".to_string()],
                    "C",
                    vec![
                        BlifCube::new(vec![lit('1'), lit('1'), lit('-')], true),
                        BlifCube::new(vec![lit('1'), lit('-'), lit('1')], true),
                        BlifCube::new(vec![lit('-'), lit('1'), lit('1')], true),
                    ],
                )
                .unwrap(),
                TwoLevelNode::new(
                    vec!["CIn".to_string(), "A".to_string(), "B".to_string()],
                    "S",
                    vec![
                        BlifCube::new(vec![lit('1'), lit('0'), lit('0')], true),
                        BlifCube::new(vec![lit('0'), lit('1'), lit('0')], true),
                        BlifCube::new(vec![lit('0'), lit('0'), lit('1')], true),
                        BlifCube::new(vec![lit('1'), lit('1'), lit('1')], true),
                    ],
                )
                .unwrap(),
            ],
        )
        .unwrap();

        let result =
            map_two_level_to_virtual_network(&model, Some(&library), ComMapOptions::default())
                .unwrap();
        let print_gate = result.network.format_print_gate().unwrap();

        assert_eq!(print_gate.lines().next(), Some("nodes=14"));
        assert_eq!(print_gate.matches(" inv ").count(), 4);
        assert_eq!(print_gate.matches(" nand2 ").count(), 8);
        assert_eq!(print_gate.matches(" nor2 ").count(), 2);
        assert!(!print_gate.contains(" and "));
        assert!(!print_gate.contains(" or "));
    }

    #[test]
    fn bridges_genlib_and_com_map_options_to_virtual_network() {
        let library = crate::map::library::parse_genlib(concat!(
            "GATE and2 1 O=a*b;\n",
            "PIN a NONINV 1 999 1 .2 1 .2\n",
            "PIN b NONINV 1 999 1 .2 1 .2\n",
        ))
        .unwrap();
        let options = crate::map::com_map::parse_map_args(["-m0"]).unwrap();
        let model = TwoLevelModel::new(
            None,
            vec!["a".to_string(), "b".to_string()],
            vec!["f".to_string()],
            vec![
                TwoLevelNode::new(
                    vec!["a".to_string(), "b".to_string()],
                    "f",
                    vec![BlifCube::new(vec![lit('1'), lit('1')], true)],
                )
                .unwrap(),
            ],
        )
        .unwrap();

        let result =
            map_two_level_with_genlib_to_virtual_network(&model, Some(&library), &options).unwrap();

        assert_eq!(
            result.network.format_print_gate().unwrap(),
            "nodes=1\n{f} and2 2 pin0=a pin1=b\n"
        );
        assert_eq!(result.options, ComMapOptions::from(&options));
    }

    #[test]
    fn full_tree_matching_request_uses_native_primitive_fallback() {
        let model = TwoLevelModel::new(
            None,
            vec!["a".to_string()],
            vec!["f".to_string()],
            vec![
                TwoLevelNode::new(
                    vec!["a".to_string()],
                    "f",
                    vec![BlifCube::new(vec![lit('1')], true)],
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let options = ComMapOptions {
            require_full_tree_matching: true,
            ..ComMapOptions::default()
        };

        let result = map_two_level_to_virtual_network(&model, None, options).unwrap();

        assert_eq!(result.network.format_print_gate().unwrap(), "nodes=0\n");
        assert!(
            result
                .diagnostics
                .contains(&MapDiagnostic::FullTreeMatchingApproximatedByPrimitiveLowering)
        );
    }
}
