//! Native Rust phase-library support.
//!
//! This module ports the library-aware rules from the SIS phase assignment
//! implementation. It keeps the original distinction between unmapped Boolean
//! phase changes and mapped gate-dual replacement, but represents network
//! rewrites as plans instead of mutating SIS `network_t` and `node_t` values.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub type NodeId = usize;
pub type GateId = usize;
pub type ClassId = usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    PrimaryInput,
    PrimaryOutput,
    Buffer,
    Inverter,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseNode {
    pub id: NodeId,
    pub function: NodeFunction,
    pub fanins: Vec<NodeId>,
    pub mapped_gate: Option<GateId>,
}

impl PhaseNode {
    pub fn new(id: NodeId, function: NodeFunction) -> Self {
        Self {
            id,
            function,
            fanins: Vec::new(),
            mapped_gate: None,
        }
    }

    pub fn with_fanins(mut self, fanins: impl Into<Vec<NodeId>>) -> Self {
        self.fanins = fanins.into();
        self
    }

    pub fn with_gate(mut self, gate: GateId) -> Self {
        self.mapped_gate = Some(gate);
        self
    }

    pub fn is_internal(&self) -> bool {
        !matches!(
            self.function,
            NodeFunction::PrimaryInput | NodeFunction::PrimaryOutput
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LibraryGate {
    pub id: GateId,
    pub class: ClassId,
    pub area: f64,
    pub input_pins: Vec<String>,
}

impl LibraryGate {
    pub fn new(id: GateId, class: ClassId, area: f64, input_pins: impl Into<Vec<String>>) -> Self {
        Self {
            id,
            class,
            area,
            input_pins: input_pins.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryClass {
    pub id: ClassId,
    pub dual: Option<ClassId>,
    pub gates: Vec<GateId>,
}

impl LibraryClass {
    pub fn new(id: ClassId, dual: Option<ClassId>, gates: impl Into<Vec<GateId>>) -> Self {
        Self {
            id,
            dual,
            gates: gates.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PhaseLibrary {
    gates: HashMap<GateId, LibraryGate>,
    classes: HashMap<ClassId, LibraryClass>,
    inverter_class: Option<ClassId>,
}

impl PhaseLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_inverter_class(mut self, class: ClassId) -> Self {
        self.inverter_class = Some(class);
        self
    }

    pub fn add_class(&mut self, class: LibraryClass) {
        self.classes.insert(class.id, class);
    }

    pub fn add_gate(&mut self, gate: LibraryGate) {
        self.gates.insert(gate.id, gate);
    }

    pub fn gate(&self, gate: GateId) -> Result<&LibraryGate, PhaseLibError> {
        self.gates
            .get(&gate)
            .ok_or(PhaseLibError::MissingGate { gate })
    }

    pub fn class(&self, class: ClassId) -> Result<&LibraryClass, PhaseLibError> {
        self.classes
            .get(&class)
            .ok_or(PhaseLibError::MissingClass { class })
    }

    pub fn min_area_gate(&self, class: ClassId) -> Result<&LibraryGate, PhaseLibError> {
        let class = self.class(class)?;
        class
            .gates
            .iter()
            .map(|gate| self.gate(*gate))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .min_by(|left, right| left.area.total_cmp(&right.area))
            .ok_or(PhaseLibError::EmptyClass { class: class.id })
    }

    fn dual_min_area_gate(&self, gate: GateId) -> Result<Option<&LibraryGate>, PhaseLibError> {
        let gate = self.gate(gate)?;
        let dual_class = self.class(gate.class)?.dual;
        dual_class
            .map(|class| self.min_area_gate(class))
            .transpose()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseLibraryState {
    keep_mapped: bool,
    inverter_gate: Option<GateId>,
}

impl PhaseLibraryState {
    pub fn unmapped() -> Self {
        Self {
            keep_mapped: false,
            inverter_gate: None,
        }
    }

    pub fn mapped(inverter_gate: GateId) -> Self {
        Self {
            keep_mapped: true,
            inverter_gate: Some(inverter_gate),
        }
    }

    pub fn keep_mapped(&self) -> bool {
        self.keep_mapped
    }

    pub fn inverter_gate(&self) -> Option<GateId> {
        self.inverter_gate
    }
}

pub fn phase_lib_setup(nodes: &[PhaseNode], library: Option<&PhaseLibrary>) -> PhaseLibraryState {
    let Some(library) = library else {
        return PhaseLibraryState::unmapped();
    };

    if nodes
        .iter()
        .any(|node| node.is_internal() && node.mapped_gate.is_none())
    {
        return PhaseLibraryState::unmapped();
    }

    let Some(inverter_class) = library.inverter_class else {
        return PhaseLibraryState::unmapped();
    };

    match library.min_area_gate(inverter_class) {
        Ok(gate) => PhaseLibraryState::mapped(gate.id),
        Err(_) => PhaseLibraryState::unmapped(),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RowData {
    pub node: PhaseNode,
    pub pos_used: i32,
    pub neg_used: i32,
    pub inv_save: i32,
    pub marked: bool,
    pub invertible: bool,
    pub inverted: bool,
    pub po: bool,
    pub area: f64,
    pub dual_area: f64,
}

impl RowData {
    pub fn new(node: PhaseNode) -> Self {
        Self {
            node,
            pos_used: 0,
            neg_used: 0,
            inv_save: 0,
            marked: false,
            invertible: false,
            inverted: false,
            po: false,
            area: 0.0,
            dual_area: 0.0,
        }
    }
}

pub fn phase_invertible_set(
    row: &mut RowData,
    state: &PhaseLibraryState,
    library: Option<&PhaseLibrary>,
) -> Result<(), PhaseLibError> {
    match row.node.function {
        NodeFunction::PrimaryInput | NodeFunction::PrimaryOutput => {
            set_uninvertible_without_area(row);
        }
        _ if !state.keep_mapped => {
            set_invertible_without_area(row);
        }
        _ => {
            let Some(gate_id) = row.node.mapped_gate else {
                set_invertible_without_area(row);
                return Ok(());
            };
            let library = library.ok_or(PhaseLibError::MappedStateWithoutLibrary)?;
            let gate = library.gate(gate_id)?;
            match library.dual_min_area_gate(gate_id)? {
                Some(dual_gate) => {
                    row.invertible = true;
                    row.area = gate.area;
                    row.dual_area = dual_gate.area;
                }
                None => {
                    set_uninvertible_without_area(row);
                }
            }
        }
    }

    Ok(())
}

fn set_invertible_without_area(row: &mut RowData) {
    row.invertible = true;
    row.area = 0.0;
    row.dual_area = 0.0;
}

fn set_uninvertible_without_area(row: &mut RowData) {
    row.invertible = false;
    row.area = 0.0;
    row.dual_area = 0.0;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeInvertStep {
    InvertBooleanNode {
        node: NodeId,
    },
    InsertInputInverter {
        source: NodeId,
        inverter_gate: GateId,
        formal: String,
    },
    ReplaceWithDualGate {
        node: NodeId,
        dual_gate: GateId,
        formals: Vec<String>,
    },
    WrapWithOutputInverter {
        node: NodeId,
        inverter_gate: GateId,
        formal: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeInvertPlan {
    pub steps: Vec<NodeInvertStep>,
}

pub fn phase_node_invert_plan(
    node: &PhaseNode,
    state: &PhaseLibraryState,
    library: Option<&PhaseLibrary>,
) -> Result<NodeInvertPlan, PhaseLibError> {
    if !state.keep_mapped {
        return Ok(NodeInvertPlan {
            steps: vec![NodeInvertStep::InvertBooleanNode { node: node.id }],
        });
    }

    let library = library.ok_or(PhaseLibError::MappedStateWithoutLibrary)?;
    let gate_id = node
        .mapped_gate
        .ok_or(PhaseLibError::MappedNodeWithoutGate { node: node.id })?;
    let dual_gate = library
        .dual_min_area_gate(gate_id)?
        .ok_or(PhaseLibError::MissingDualClassForGate { gate: gate_id })?;
    let inverter_gate_id = state
        .inverter_gate
        .ok_or(PhaseLibError::MappedStateWithoutInverter)?;
    let inverter_gate = library.gate(inverter_gate_id)?;
    let inverter_formal =
        inverter_gate
            .input_pins
            .first()
            .cloned()
            .ok_or(PhaseLibError::MissingGateInputPin {
                gate: inverter_gate_id,
                pin: 0,
            })?;

    let mut steps = Vec::with_capacity(node.fanins.len() + 2);
    for fanin in &node.fanins {
        steps.push(NodeInvertStep::InsertInputInverter {
            source: *fanin,
            inverter_gate: inverter_gate_id,
            formal: inverter_formal.clone(),
        });
    }

    let formals = node
        .fanins
        .iter()
        .enumerate()
        .map(|(index, _)| {
            dual_gate
                .input_pins
                .get(index)
                .cloned()
                .ok_or(PhaseLibError::MissingGateInputPin {
                    gate: dual_gate.id,
                    pin: index,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    steps.push(NodeInvertStep::ReplaceWithDualGate {
        node: node.id,
        dual_gate: dual_gate.id,
        formals,
    });
    steps.push(NodeInvertStep::WrapWithOutputInverter {
        node: node.id,
        inverter_gate: inverter_gate_id,
        formal: inverter_formal,
    });

    Ok(NodeInvertPlan { steps })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodePhase {
    pub row: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetPhase {
    pub rows: Vec<RowData>,
    pub cost: f64,
}

impl NetPhase {
    pub fn new(rows: Vec<RowData>) -> Self {
        Self { rows, cost: 0.0 }
    }
}

pub fn phase_value(
    node_phase: &NodePhase,
    net_phase: &NetPhase,
    state: &PhaseLibraryState,
    library: Option<&PhaseLibrary>,
) -> Result<f64, PhaseLibError> {
    let row = net_phase
        .rows
        .get(node_phase.row)
        .ok_or(PhaseLibError::MissingRow {
            row: node_phase.row,
        })?;

    if !state.keep_mapped {
        return Ok(f64::from(row.inv_save));
    }

    let library = library.ok_or(PhaseLibError::MappedStateWithoutLibrary)?;
    let gate_id = row
        .node
        .mapped_gate
        .ok_or(PhaseLibError::MappedNodeWithoutGate { node: row.node.id })?;
    let gate = library.gate(gate_id)?;
    let dual_gate = library
        .dual_min_area_gate(gate_id)?
        .ok_or(PhaseLibError::MissingDualClassForGate { gate: gate_id })?;
    let inverter_gate_id = state
        .inverter_gate
        .ok_or(PhaseLibError::MappedStateWithoutInverter)?;
    let inverter_gate = library.gate(inverter_gate_id)?;

    let phase_delta = if row.inverted {
        dual_gate.area - gate.area
    } else {
        gate.area - dual_gate.area
    };

    Ok(phase_delta + f64::from(row.inv_save) * inverter_gate.area)
}

pub fn cost_comp(
    net_phase: &NetPhase,
    state: &PhaseLibraryState,
    library: Option<&PhaseLibrary>,
) -> Result<f64, PhaseLibError> {
    let inverter_area = if state.keep_mapped {
        let library = library.ok_or(PhaseLibError::MappedStateWithoutLibrary)?;
        let inverter_gate_id = state
            .inverter_gate
            .ok_or(PhaseLibError::MappedStateWithoutInverter)?;
        library.gate(inverter_gate_id)?.area
    } else {
        1.0
    };

    let mut count = 0.0;
    for row in &net_phase.rows {
        count += if row.inverted {
            row.dual_area
        } else {
            row.area
        };

        if row.pos_used != 0 {
            count += inverter_area;
        }
    }

    Ok(count)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhaseRecordCleanup {
    RemoveMappedInverters,
    SweepAndAddInverters,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseRecordPlan {
    pub inverted_nodes: Vec<NodeId>,
    pub cleanup: PhaseRecordCleanup,
}

pub fn phase_record_plan(net_phase: &NetPhase, state: &PhaseLibraryState) -> PhaseRecordPlan {
    PhaseRecordPlan {
        inverted_nodes: net_phase
            .rows
            .iter()
            .filter(|row| row.inverted)
            .map(|row| row.node.id)
            .collect(),
        cleanup: if state.keep_mapped {
            PhaseRecordCleanup::RemoveMappedInverters
        } else {
            PhaseRecordCleanup::SweepAndAddInverters
        },
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhaseLibError {
    EmptyClass { class: ClassId },
    MappedNodeWithoutGate { node: NodeId },
    MappedStateWithoutInverter,
    MappedStateWithoutLibrary,
    MissingClass { class: ClassId },
    MissingDualClassForGate { gate: GateId },
    MissingGate { gate: GateId },
    MissingGateInputPin { gate: GateId, pin: usize },
    MissingRow { row: usize },
}

impl fmt::Display for PhaseLibError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyClass { class } => write!(f, "library class {class} has no gates"),
            Self::MappedNodeWithoutGate { node } => {
                write!(
                    f,
                    "mapped phase operation requires node {node} to have a gate"
                )
            }
            Self::MappedStateWithoutInverter => {
                write!(f, "mapped phase operation requires an inverter gate")
            }
            Self::MappedStateWithoutLibrary => {
                write!(f, "mapped phase operation requires a library")
            }
            Self::MissingClass { class } => write!(f, "library class {class} was not found"),
            Self::MissingDualClassForGate { gate } => {
                write!(f, "gate {gate} has no dual class")
            }
            Self::MissingGate { gate } => write!(f, "library gate {gate} was not found"),
            Self::MissingGateInputPin { gate, pin } => {
                write!(f, "library gate {gate} has no input pin {pin}")
            }
            Self::MissingRow { row } => write!(f, "phase row {row} was not found"),
        }
    }
}

impl Error for PhaseLibError {}

#[cfg(test)]
mod tests {
    use super::*;

    const INV_CLASS: ClassId = 10;
    const AND_CLASS: ClassId = 20;
    const NAND_CLASS: ClassId = 21;
    const INV_GATE: GateId = 1;
    const SLOW_INV_GATE: GateId = 2;
    const AND_GATE: GateId = 3;
    const NAND_GATE: GateId = 4;

    fn pin_names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    fn sample_library() -> PhaseLibrary {
        let mut library = PhaseLibrary::new().with_inverter_class(INV_CLASS);
        library.add_class(LibraryClass::new(
            INV_CLASS,
            None,
            vec![SLOW_INV_GATE, INV_GATE],
        ));
        library.add_class(LibraryClass::new(
            AND_CLASS,
            Some(NAND_CLASS),
            vec![AND_GATE],
        ));
        library.add_class(LibraryClass::new(
            NAND_CLASS,
            Some(AND_CLASS),
            vec![NAND_GATE],
        ));
        library.add_gate(LibraryGate::new(
            SLOW_INV_GATE,
            INV_CLASS,
            3.0,
            pin_names(&["a"]),
        ));
        library.add_gate(LibraryGate::new(
            INV_GATE,
            INV_CLASS,
            1.5,
            pin_names(&["a"]),
        ));
        library.add_gate(LibraryGate::new(
            AND_GATE,
            AND_CLASS,
            5.0,
            pin_names(&["a", "b"]),
        ));
        library.add_gate(LibraryGate::new(
            NAND_GATE,
            NAND_CLASS,
            4.0,
            pin_names(&["na", "nb"]),
        ));
        library
    }

    #[test]
    fn setup_keeps_mapping_only_when_all_internal_nodes_are_mapped_and_inverter_exists() {
        let library = sample_library();
        let nodes = vec![
            PhaseNode::new(1, NodeFunction::PrimaryInput),
            PhaseNode::new(2, NodeFunction::Other).with_gate(AND_GATE),
        ];

        let state = phase_lib_setup(&nodes, Some(&library));

        assert!(state.keep_mapped());
        assert_eq!(state.inverter_gate(), Some(INV_GATE));
    }

    #[test]
    fn setup_falls_back_to_boolean_phase_when_internal_node_is_unmapped() {
        let library = sample_library();
        let nodes = vec![PhaseNode::new(2, NodeFunction::Other)];

        assert_eq!(
            phase_lib_setup(&nodes, Some(&library)),
            PhaseLibraryState::unmapped()
        );
    }

    #[test]
    fn invertible_set_records_gate_and_dual_area_for_mapped_node() {
        let library = sample_library();
        let state = PhaseLibraryState::mapped(INV_GATE);
        let mut row = RowData::new(PhaseNode::new(2, NodeFunction::Other).with_gate(AND_GATE));

        phase_invertible_set(&mut row, &state, Some(&library)).unwrap();

        assert!(row.invertible);
        assert_eq!(row.area, 5.0);
        assert_eq!(row.dual_area, 4.0);
    }

    #[test]
    fn primary_inputs_and_outputs_are_never_invertible() {
        let state = PhaseLibraryState::unmapped();
        let mut input = RowData::new(PhaseNode::new(1, NodeFunction::PrimaryInput));
        let mut output = RowData::new(PhaseNode::new(2, NodeFunction::PrimaryOutput));

        phase_invertible_set(&mut input, &state, None).unwrap();
        phase_invertible_set(&mut output, &state, None).unwrap();

        assert!(!input.invertible);
        assert!(!output.invertible);
        assert_eq!(input.area, 0.0);
        assert_eq!(output.dual_area, 0.0);
    }

    #[test]
    fn mapped_node_invert_plan_wraps_fanins_dual_gate_and_output_inverter() {
        let library = sample_library();
        let state = PhaseLibraryState::mapped(INV_GATE);
        let node = PhaseNode::new(7, NodeFunction::Other)
            .with_fanins(vec![1, 2])
            .with_gate(AND_GATE);

        let plan = phase_node_invert_plan(&node, &state, Some(&library)).unwrap();

        assert_eq!(
            plan.steps,
            vec![
                NodeInvertStep::InsertInputInverter {
                    source: 1,
                    inverter_gate: INV_GATE,
                    formal: "a".to_owned(),
                },
                NodeInvertStep::InsertInputInverter {
                    source: 2,
                    inverter_gate: INV_GATE,
                    formal: "a".to_owned(),
                },
                NodeInvertStep::ReplaceWithDualGate {
                    node: 7,
                    dual_gate: NAND_GATE,
                    formals: pin_names(&["na", "nb"]),
                },
                NodeInvertStep::WrapWithOutputInverter {
                    node: 7,
                    inverter_gate: INV_GATE,
                    formal: "a".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn phase_value_matches_unmapped_inverter_saving() {
        let mut row = RowData::new(PhaseNode::new(2, NodeFunction::Other));
        row.inv_save = 3;
        let net = NetPhase::new(vec![row]);
        let phase = NodePhase { row: 0 };

        assert_eq!(
            phase_value(&phase, &net, &PhaseLibraryState::unmapped(), None).unwrap(),
            3.0
        );
    }

    #[test]
    fn phase_value_uses_dual_area_delta_and_saved_inverter_area_for_mapped_nodes() {
        let library = sample_library();
        let state = PhaseLibraryState::mapped(INV_GATE);
        let mut row = RowData::new(PhaseNode::new(2, NodeFunction::Other).with_gate(AND_GATE));
        row.inv_save = 2;
        let net = NetPhase::new(vec![row]);
        let phase = NodePhase { row: 0 };

        assert_eq!(
            phase_value(&phase, &net, &state, Some(&library)).unwrap(),
            4.0
        );
    }

    #[test]
    fn cost_comp_counts_selected_phase_area_and_positive_use_inverter_cost() {
        let library = sample_library();
        let state = PhaseLibraryState::mapped(INV_GATE);
        let mut normal = RowData::new(PhaseNode::new(2, NodeFunction::Other).with_gate(AND_GATE));
        normal.area = 5.0;
        normal.dual_area = 4.0;
        normal.pos_used = 1;
        let mut inverted =
            RowData::new(PhaseNode::new(3, NodeFunction::Other).with_gate(NAND_GATE));
        inverted.area = 4.0;
        inverted.dual_area = 5.0;
        inverted.inverted = true;
        let net = NetPhase::new(vec![normal, inverted]);

        assert_eq!(cost_comp(&net, &state, Some(&library)).unwrap(), 11.5);
    }

    #[test]
    fn record_plan_keeps_only_inverted_nodes_and_selects_cleanup_mode() {
        let mut first = RowData::new(PhaseNode::new(1, NodeFunction::Other));
        first.inverted = true;
        let second = RowData::new(PhaseNode::new(2, NodeFunction::Other));
        let net = NetPhase::new(vec![first, second]);

        assert_eq!(
            phase_record_plan(&net, &PhaseLibraryState::unmapped()),
            PhaseRecordPlan {
                inverted_nodes: vec![1],
                cleanup: PhaseRecordCleanup::SweepAndAddInverters,
            }
        );
    }
}
