//! Owned-data writer for SIS networks destined for an OCT-like symbolic facet.
//!
//! The original implementation writes directly through the OCT C library.  This
//! native version keeps the observable write decisions in Rust data structures:
//! policy properties, formal terminals, nets, instances, generic masters, latch
//! instances, and clock event metadata.  A later integration layer can translate
//! `OctWrite` into a concrete persistence format without reintroducing per-file
//! C entry points.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OctWriteOptions {
    pub technology: String,
    pub editstyle: String,
    pub viewtype: String,
    pub cell_view: String,
    pub cell_path: Option<String>,
}

impl Default for OctWriteOptions {
    fn default() -> Self {
        Self {
            technology: "scmos".to_string(),
            editstyle: "SYMBOLIC".to_string(),
            viewtype: "SYMBOLIC".to_string(),
            cell_view: "physical".to_string(),
            cell_path: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SisNetwork {
    pub nodes: Vec<SisNode>,
    pub latches: Vec<SisLatch>,
    pub clocks: Vec<SisClock>,
    pub cycle_time: Option<OrderedReal>,
}

impl SisNetwork {
    pub fn new(nodes: Vec<SisNode>) -> Self {
        Self {
            nodes,
            latches: Vec::new(),
            clocks: Vec::new(),
            cycle_time: None,
        }
    }

    pub fn with_latches(mut self, latches: Vec<SisLatch>) -> Self {
        self.latches = latches;
        self
    }

    pub fn with_clocks(mut self, clocks: Vec<SisClock>, cycle_time: Option<f64>) -> Self {
        self.clocks = clocks;
        self.cycle_time = cycle_time.map(OrderedReal);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SisNode {
    pub name: String,
    pub kind: SisNodeKind,
    pub fanins: Vec<String>,
    pub function: NodeFunction,
    pub gate: Option<GateBinding>,
    pub is_real_interface: bool,
    pub is_clock_input: bool,
}

impl SisNode {
    pub fn primary_input(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name,
            kind: SisNodeKind::PrimaryInput,
            fanins: Vec::new(),
            function: NodeFunction::PrimaryInput,
            gate: None,
            is_real_interface: true,
            is_clock_input: false,
        }
    }

    pub fn primary_output(name: impl Into<String>, fanin: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: SisNodeKind::PrimaryOutput,
            fanins: vec![fanin.into()],
            function: NodeFunction::PrimaryOutput,
            gate: None,
            is_real_interface: true,
            is_clock_input: false,
        }
    }

    pub fn internal(
        name: impl Into<String>,
        fanins: impl IntoIterator<Item = impl Into<String>>,
        function: NodeFunction,
    ) -> Self {
        Self {
            name: name.into(),
            kind: SisNodeKind::Internal,
            fanins: fanins.into_iter().map(Into::into).collect(),
            function,
            gate: None,
            is_real_interface: true,
            is_clock_input: false,
        }
    }

    pub fn with_gate(mut self, gate: GateBinding) -> Self {
        self.gate = Some(gate);
        self
    }

    pub fn with_real_interface(mut self, is_real_interface: bool) -> Self {
        self.is_real_interface = is_real_interface;
        self
    }

    pub fn with_clock_input(mut self, is_clock_input: bool) -> Self {
        self.is_clock_input = is_clock_input;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SisNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    PrimaryInput,
    PrimaryOutput,
    Undefined,
    Zero,
    One,
    Sop {
        fanins: Vec<String>,
        cubes: Vec<Vec<Literal>>,
    },
    Factored(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Literal {
    pub fanin: String,
    pub polarity: LiteralPolarity,
}

impl Literal {
    pub fn positive(fanin: impl Into<String>) -> Self {
        Self {
            fanin: fanin.into(),
            polarity: LiteralPolarity::Positive,
        }
    }

    pub fn negative(fanin: impl Into<String>) -> Self {
        Self {
            fanin: fanin.into(),
            polarity: LiteralPolarity::Negative,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralPolarity {
    Positive,
    Negative,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateBinding {
    pub cell: String,
    pub view: Option<String>,
    pub input_pins: Vec<String>,
    pub output_pins: Vec<String>,
    pub clock_pins: Vec<String>,
    pub gate_name: Option<String>,
    pub gate_type: GateType,
}

impl GateBinding {
    pub fn combinational(
        cell: impl Into<String>,
        input_pins: impl IntoIterator<Item = impl Into<String>>,
        output_pin: impl Into<String>,
    ) -> Self {
        Self {
            cell: cell.into(),
            view: None,
            input_pins: input_pins.into_iter().map(Into::into).collect(),
            output_pins: vec![output_pin.into()],
            clock_pins: Vec::new(),
            gate_name: None,
            gate_type: GateType::Combinational,
        }
    }

    pub fn with_view(mut self, view: impl Into<String>) -> Self {
        self.view = Some(view.into());
        self
    }

    pub fn with_gate_name(mut self, gate_name: impl Into<String>) -> Self {
        self.gate_name = Some(gate_name.into());
        self
    }

    pub fn with_clock_pins(
        mut self,
        clock_pins: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.clock_pins = clock_pins.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_gate_type(mut self, gate_type: GateType) -> Self {
        self.gate_type = gate_type;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateType {
    Combinational,
    Sequential,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SisLatch {
    pub input_output: String,
    pub output_node: String,
    pub data_node: String,
    pub control_output: Option<String>,
    pub initial_value: i32,
    pub gate: GateBinding,
    pub feedback_pin: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SisClock {
    pub name: String,
    pub rise: Option<ClockEventSpec>,
    pub fall: Option<ClockEventSpec>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockEventSpec {
    pub nominal_position: OrderedReal,
    pub lower_range: OrderedReal,
    pub upper_range: OrderedReal,
    pub dependencies: Vec<ClockEdgeSpec>,
}

impl ClockEventSpec {
    pub fn new(nominal_position: f64, lower_range: f64, upper_range: f64) -> Self {
        Self {
            nominal_position: OrderedReal(nominal_position),
            lower_range: OrderedReal(lower_range),
            upper_range: OrderedReal(upper_range),
            dependencies: Vec::new(),
        }
    }

    pub fn with_dependencies(mut self, dependencies: Vec<ClockEdgeSpec>) -> Self {
        self.dependencies = dependencies;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockEdgeSpec {
    pub clock_name: String,
    pub transition: ClockTransition,
    pub lower_range: OrderedReal,
    pub upper_range: OrderedReal,
}

impl ClockEdgeSpec {
    pub fn new(
        clock_name: impl Into<String>,
        transition: ClockTransition,
        lower_range: f64,
        upper_range: f64,
    ) -> Self {
        Self {
            clock_name: clock_name.into(),
            transition,
            lower_range: OrderedReal(lower_range),
            upper_range: OrderedReal(upper_range),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ClockTransition {
    Rise,
    Fall,
}

#[derive(Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct OrderedReal(pub f64);

impl Eq for OrderedReal {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OctWrite {
    pub contents: OctFacet,
    pub interface: OctFacet,
    pub generic_masters: Vec<GenericMaster>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OctFacet {
    pub cell: String,
    pub view: String,
    pub facet: String,
    pub properties: BTreeMap<String, OctValue>,
    pub terminals: Vec<OctTerminal>,
    pub nets: Vec<OctNet>,
    pub instances: Vec<OctInstance>,
    pub bags: Vec<OctBag>,
}

impl OctFacet {
    fn new(cell: impl Into<String>, view: impl Into<String>, facet: impl Into<String>) -> Self {
        Self {
            cell: cell.into(),
            view: view.into(),
            facet: facet.into(),
            properties: BTreeMap::new(),
            terminals: Vec::new(),
            nets: Vec::new(),
            instances: Vec::new(),
            bags: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OctTerminal {
    pub name: String,
    pub properties: BTreeMap<String, OctValue>,
}

impl OctTerminal {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            properties: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OctNet {
    pub name: String,
    pub properties: BTreeMap<String, OctValue>,
    pub attachments: Vec<OctAttachment>,
}

impl OctNet {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            properties: BTreeMap::new(),
            attachments: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OctAttachment {
    FormalTerminal(String),
    InstanceTerminal { instance: String, terminal: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OctInstance {
    pub name: String,
    pub master_cell: String,
    pub master_view: String,
    pub terminals: Vec<OctInstanceTerminal>,
    pub properties: BTreeMap<String, OctValue>,
}

impl OctInstance {
    fn new(
        name: impl Into<String>,
        master_cell: impl Into<String>,
        master_view: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            master_cell: master_cell.into(),
            master_view: master_view.into(),
            terminals: Vec::new(),
            properties: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OctInstanceTerminal {
    pub name: String,
    pub net: String,
    pub properties: BTreeMap<String, OctValue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OctBag {
    pub name: String,
    pub members: Vec<OctBagMember>,
    pub properties: BTreeMap<String, OctValue>,
    pub bags: Vec<OctBag>,
}

impl OctBag {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            members: Vec::new(),
            properties: BTreeMap::new(),
            bags: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OctBagMember {
    Instance(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OctValue {
    String(String),
    Integer(i32),
    Real(OrderedReal),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenericMaster {
    pub cell: String,
    pub view: String,
    pub input_count: usize,
    pub contents: OctFacet,
    pub interface: OctFacet,
}

pub fn write_oct(
    network: &SisNetwork,
    cell: impl Into<String>,
    view: impl Into<String>,
    options: &OctWriteOptions,
) -> Result<OctWrite, OctWriteError> {
    let cell = cell.into();
    let view = view.into();
    let mut writer = OctWriter::new(network, cell, view, options)?;
    writer.write()
}

pub fn unpack_cell_view(name: &str, default_view: &str) -> (String, String) {
    match name.split_once(':') {
        Some((cell, view)) => (cell.to_string(), view.to_string()),
        None => (name.to_string(), default_view.to_string()),
    }
}

pub fn generic_pin_name(index: usize) -> String {
    format!("in{index}")
}

pub fn generic_output_pin_name(index: usize) -> String {
    format!("out{index}")
}

pub fn logic_function_property(node: &SisNode) -> Result<Option<String>, OctWriteError> {
    match &node.function {
        NodeFunction::Undefined => Err(OctWriteError::UndefinedFunction {
            node: node.name.clone(),
        }),
        NodeFunction::Zero => Ok(Some("(0)".to_string())),
        NodeFunction::One => Ok(Some("(1)".to_string())),
        NodeFunction::Sop { fanins, cubes } => format_sop_factored(fanins, cubes).map(Some),
        NodeFunction::Factored(value) => Ok(Some(value.clone())),
        NodeFunction::PrimaryInput | NodeFunction::PrimaryOutput => Ok(None),
    }
}

fn format_sop_factored(fanins: &[String], cubes: &[Vec<Literal>]) -> Result<String, OctWriteError> {
    if cubes.is_empty() {
        return Ok(String::new());
    }

    let fanin_set: HashSet<&str> = fanins.iter().map(String::as_str).collect();
    for cube in cubes {
        for literal in cube {
            if !fanin_set.contains(literal.fanin.as_str()) {
                return Err(OctWriteError::UnknownLiteral {
                    fanin: literal.fanin.clone(),
                });
            }
        }
    }

    if cubes.len() == 1 {
        return Ok(format_cube(&cubes[0]));
    }

    let mut value = "(+".to_string();
    for cube in cubes {
        value.push(' ');
        value.push_str(&format_cube(cube));
    }
    value.push(')');
    Ok(value)
}

fn format_cube(cube: &[Literal]) -> String {
    if cube.is_empty() {
        return String::new();
    }

    if cube.len() == 1 {
        return format_literal(&cube[0]);
    }

    let mut value = "(*".to_string();
    for literal in cube {
        value.push(' ');
        value.push_str(&format_literal(literal));
    }
    value.push(')');
    value
}

fn format_literal(literal: &Literal) -> String {
    match literal.polarity {
        LiteralPolarity::Positive => literal.fanin.clone(),
        LiteralPolarity::Negative => format!("(! {})", literal.fanin),
    }
}

struct OctWriter<'a> {
    network: &'a SisNetwork,
    options: &'a OctWriteOptions,
    contents: OctFacet,
    node_index: HashMap<String, &'a SisNode>,
    net_positions: HashMap<String, usize>,
    generic_masters: BTreeMap<usize, GenericMaster>,
}

impl<'a> OctWriter<'a> {
    fn new(
        network: &'a SisNetwork,
        cell: String,
        view: String,
        options: &'a OctWriteOptions,
    ) -> Result<Self, OctWriteError> {
        let mut node_index = HashMap::with_capacity(network.nodes.len());
        for node in &network.nodes {
            if node_index.insert(node.name.clone(), node).is_some() {
                return Err(OctWriteError::DuplicateNode {
                    node: node.name.clone(),
                });
            }
        }

        for node in &network.nodes {
            for fanin in &node.fanins {
                if !node_index.contains_key(fanin) {
                    return Err(OctWriteError::MissingFanin {
                        node: node.name.clone(),
                        fanin: fanin.clone(),
                    });
                }
            }
        }

        for latch in &network.latches {
            require_node(&node_index, &latch.input_output)?;
            require_node(&node_index, &latch.output_node)?;
            require_node(&node_index, &latch.data_node)?;
            if let Some(control_output) = &latch.control_output {
                require_node(&node_index, control_output)?;
            }
        }

        Ok(Self {
            network,
            options,
            contents: OctFacet::new(cell, view, "contents"),
            node_index,
            net_positions: HashMap::new(),
            generic_masters: BTreeMap::new(),
        })
    }

    fn write(&mut self) -> Result<OctWrite, OctWriteError> {
        attach_policy_properties(&mut self.contents, self.options);
        self.contents.properties.insert(
            "CELLCLASS".to_string(),
            OctValue::String("MODULE".to_string()),
        );
        self.contents.bags.push(OctBag::new("INSTANCES"));
        self.write_clocks();

        let latch_inputs: HashSet<&str> = self
            .network
            .latches
            .iter()
            .map(|latch| latch.input_output.as_str())
            .collect();
        let latch_outputs: HashMap<&str, &SisLatch> = self
            .network
            .latches
            .iter()
            .map(|latch| (latch.output_node.as_str(), latch))
            .collect();

        for node_name in self.dfs_order()? {
            let node = self.node_index[&node_name];
            if latch_inputs.contains(node.name.as_str()) && node.kind == SisNodeKind::PrimaryOutput
            {
                continue;
            }

            if let Some(latch) = latch_outputs.get(node.name.as_str()) {
                self.create_latch(latch)?;
                continue;
            }

            match node.kind {
                SisNodeKind::PrimaryInput => {
                    if node.is_real_interface {
                        self.create_primary_input(node);
                    }
                }
                SisNodeKind::PrimaryOutput => {
                    if node.is_real_interface {
                        self.create_primary_output(node)?;
                    }
                }
                SisNodeKind::Internal => {
                    if node
                        .gate
                        .as_ref()
                        .is_some_and(|gate| gate.gate_type != GateType::Combinational)
                    {
                        continue;
                    }
                    self.create_instance(node)?;
                }
            }
        }

        let interface = create_interface_from_contents(&self.contents, self.options);
        let mut generic_masters: Vec<GenericMaster> =
            self.generic_masters.values().cloned().collect();
        generic_masters.sort_by_key(|master| master.input_count);

        Ok(OctWrite {
            contents: self.contents.clone(),
            interface,
            generic_masters,
        })
    }

    fn dfs_order(&self) -> Result<Vec<String>, OctWriteError> {
        let mut order = Vec::new();
        let mut marks = HashMap::new();

        let mut roots: Vec<&SisNode> = self
            .network
            .nodes
            .iter()
            .filter(|node| node.kind == SisNodeKind::PrimaryOutput)
            .collect();

        let driven: HashSet<&str> = self
            .network
            .nodes
            .iter()
            .flat_map(|node| node.fanins.iter().map(String::as_str))
            .collect();
        roots.extend(self.network.nodes.iter().filter(|node| {
            node.kind != SisNodeKind::PrimaryOutput && !driven.contains(node.name.as_str())
        }));

        for root in roots {
            self.visit(root.name.as_str(), &mut marks, &mut order)?;
        }

        Ok(order)
    }

    fn visit(
        &self,
        node_name: &str,
        marks: &mut HashMap<String, VisitState>,
        order: &mut Vec<String>,
    ) -> Result<(), OctWriteError> {
        if let Some(mark) = marks.get(node_name) {
            return match mark {
                VisitState::Active => Err(OctWriteError::CycleDetected {
                    node: node_name.to_string(),
                }),
                VisitState::Done => Ok(()),
            };
        }

        marks.insert(node_name.to_string(), VisitState::Active);
        let node = self.node_index[node_name];
        for fanin in &node.fanins {
            self.visit(fanin, marks, order)?;
        }
        marks.insert(node_name.to_string(), VisitState::Done);
        order.push(node_name.to_string());
        Ok(())
    }

    fn create_primary_input(&mut self, node: &SisNode) {
        let mut term = OctTerminal::new(&node.name);
        let mut net = self.get_or_create_net(&node.name);
        if node.is_clock_input {
            net.properties
                .insert("NETTYPE".to_string(), OctValue::String("CLOCK".to_string()));
            attach_term_properties(&mut term, "INPUT", "CLOCK");
        } else {
            attach_term_properties(&mut term, "INPUT", "SIGNAL");
        }
        net.attachments
            .push(OctAttachment::FormalTerminal(node.name.clone()));
        self.contents.terminals.push(term);
        self.replace_net(net);
    }

    fn create_primary_output(&mut self, node: &SisNode) -> Result<(), OctWriteError> {
        let fanin =
            node.fanins
                .first()
                .ok_or_else(|| OctWriteError::PrimaryOutputWithoutFanin {
                    node: node.name.clone(),
                })?;
        let mut term = OctTerminal::new(&node.name);
        attach_term_properties(&mut term, "OUTPUT", "SIGNAL");
        let mut net = self.get_or_create_net(fanin);
        net.attachments
            .push(OctAttachment::FormalTerminal(node.name.clone()));
        self.contents.terminals.push(term);
        self.replace_net(net);
        Ok(())
    }

    fn create_instance(&mut self, node: &SisNode) -> Result<(), OctWriteError> {
        let resolved = self.resolve_master(node)?;
        let mut instance = OctInstance::new(&node.name, resolved.cell, resolved.view);

        for (index, fanin) in node.fanins.iter().enumerate() {
            let pin = resolved.input_pins.get(index).cloned().ok_or_else(|| {
                OctWriteError::MissingPin {
                    node: node.name.clone(),
                    pin_index: index,
                }
            })?;
            self.connect_instance_terminal(&mut instance, &pin, fanin);
        }

        let output_pin = resolved.output_pins.first().cloned().ok_or_else(|| {
            OctWriteError::MissingOutputPin {
                node: node.name.clone(),
            }
        })?;
        self.connect_instance_terminal(&mut instance, &output_pin, &node.name);

        if resolved.is_generic {
            if let Some(value) = logic_function_property(node)? {
                if let Some(term) = instance
                    .terminals
                    .iter_mut()
                    .find(|term| term.name == output_pin)
                {
                    term.properties
                        .insert("LOGICFUNCTION".to_string(), OctValue::String(value));
                }
            }
        }

        self.attach_instance(instance);
        Ok(())
    }

    fn create_latch(&mut self, latch: &SisLatch) -> Result<(), OctWriteError> {
        let data_node = self.node_index[&latch.data_node];
        let mut instance = OctInstance::new(
            latch
                .gate
                .gate_name
                .as_deref()
                .unwrap_or(latch.output_node.as_str()),
            self.resolve_cell(&latch.gate.cell),
            latch
                .gate
                .view
                .clone()
                .unwrap_or_else(|| self.options.cell_view.clone()),
        );

        for (index, fanin) in data_node.fanins.iter().enumerate() {
            if latch.feedback_pin == Some(index) {
                continue;
            }
            let pin = latch
                .gate
                .input_pins
                .get(index)
                .ok_or_else(|| OctWriteError::MissingPin {
                    node: latch.data_node.clone(),
                    pin_index: index,
                })?
                .clone();
            self.connect_instance_terminal(&mut instance, &pin, fanin);
        }

        let output_pin = latch
            .gate
            .output_pins
            .first()
            .ok_or_else(|| OctWriteError::MissingOutputPin {
                node: latch.output_node.clone(),
            })?
            .clone();
        self.connect_instance_terminal(&mut instance, &output_pin, &latch.output_node);

        if let Some(control_output) = &latch.control_output {
            let control = self.node_index[control_output.as_str()];
            let control_fanin =
                control
                    .fanins
                    .first()
                    .ok_or_else(|| OctWriteError::PrimaryOutputWithoutFanin {
                        node: control.name.clone(),
                    })?;
            let clock_pin = latch
                .gate
                .clock_pins
                .first()
                .ok_or_else(|| OctWriteError::MissingClockPin {
                    node: latch.output_node.clone(),
                })?
                .clone();
            self.connect_instance_terminal(&mut instance, &clock_pin, control_fanin);
        }

        instance.properties.insert(
            "INITIAL_VALUE".to_string(),
            OctValue::Integer(latch.initial_value),
        );
        self.attach_instance(instance);
        Ok(())
    }

    fn resolve_master(&mut self, node: &SisNode) -> Result<ResolvedMaster, OctWriteError> {
        if let Some(gate) = &node.gate {
            if gate.input_pins.len() < node.fanins.len() {
                return Err(OctWriteError::TooFewPins {
                    node: node.name.clone(),
                    pins: gate.input_pins.len(),
                    fanins: node.fanins.len(),
                });
            }

            return Ok(ResolvedMaster {
                cell: self.resolve_cell(&gate.cell),
                view: gate
                    .view
                    .clone()
                    .unwrap_or_else(|| self.options.cell_view.clone()),
                input_pins: gate.input_pins.clone(),
                output_pins: gate.output_pins.clone(),
                is_generic: false,
            });
        }

        let input_count = node.fanins.len();
        self.ensure_generic_master(input_count);
        Ok(ResolvedMaster {
            cell: format!("{}/generic-{input_count}", self.contents.cell),
            view: self.options.cell_view.clone(),
            input_pins: (0..input_count).map(generic_pin_name).collect(),
            output_pins: vec![generic_output_pin_name(0)],
            is_generic: true,
        })
    }

    fn ensure_generic_master(&mut self, input_count: usize) {
        if self.generic_masters.contains_key(&input_count) {
            return;
        }

        let cell = format!("{}/generic-{input_count}", self.contents.cell);
        let mut contents = OctFacet::new(&cell, &self.options.cell_view, "contents");
        attach_policy_properties(&mut contents, self.options);
        contents.properties.insert(
            "CELLCLASS".to_string(),
            OctValue::String("LEAF".to_string()),
        );
        contents.properties.insert(
            "CELLTYPE".to_string(),
            OctValue::String("COMBINATIONAL".to_string()),
        );

        for index in 0..input_count {
            let mut term = OctTerminal::new(generic_pin_name(index));
            attach_term_properties(&mut term, "INPUT", "SIGNAL");
            contents.terminals.push(term);
        }
        let mut output = OctTerminal::new(generic_output_pin_name(0));
        attach_term_properties(&mut output, "OUTPUT", "SIGNAL");
        contents.terminals.push(output);

        let interface = create_interface_from_contents(&contents, self.options);
        self.generic_masters.insert(
            input_count,
            GenericMaster {
                cell,
                view: self.options.cell_view.clone(),
                input_count,
                contents,
                interface,
            },
        );
    }

    fn resolve_cell(&self, cell: &str) -> String {
        if cell.starts_with('/') || cell.starts_with('~') {
            return cell.to_string();
        }

        match &self.options.cell_path {
            Some(path) => format!("{path}/{cell}"),
            None => cell.to_string(),
        }
    }

    fn connect_instance_terminal(
        &mut self,
        instance: &mut OctInstance,
        terminal_name: &str,
        net_name: &str,
    ) {
        let mut net = self.get_or_create_net(net_name);
        net.attachments.push(OctAttachment::InstanceTerminal {
            instance: instance.name.clone(),
            terminal: terminal_name.to_string(),
        });
        instance.terminals.push(OctInstanceTerminal {
            name: terminal_name.to_string(),
            net: net_name.to_string(),
            properties: BTreeMap::new(),
        });
        self.replace_net(net);
    }

    fn get_or_create_net(&mut self, name: &str) -> OctNet {
        if let Some(position) = self.net_positions.get(name).copied() {
            return self.contents.nets[position].clone();
        }

        let net = OctNet::new(name);
        let position = self.contents.nets.len();
        self.contents.nets.push(net.clone());
        self.net_positions.insert(name.to_string(), position);
        net
    }

    fn replace_net(&mut self, net: OctNet) {
        if let Some(position) = self.net_positions.get(net.name.as_str()).copied() {
            self.contents.nets[position] = net;
        }
    }

    fn attach_instance(&mut self, instance: OctInstance) {
        if let Some(bag) = self
            .contents
            .bags
            .iter_mut()
            .find(|bag| bag.name == "INSTANCES")
        {
            bag.members
                .push(OctBagMember::Instance(instance.name.clone()));
        }
        self.contents.instances.push(instance);
    }

    fn write_clocks(&mut self) {
        if self.network.clocks.is_empty() && self.network.cycle_time.is_none() {
            return;
        }

        let mut sis_bag = OctBag::new("SIS_CLOCKS");
        if let Some(cycle_time) = self.network.cycle_time {
            if cycle_time.0 > 0.0 {
                sis_bag
                    .properties
                    .insert("CYCLETIME".to_string(), OctValue::Real(cycle_time));
            }
        }

        let mut done = BTreeSet::new();
        for clock in &self.network.clocks {
            if !done.contains(&(clock.name.clone(), ClockTransition::Rise)) {
                if let Some(rise) = &clock.rise {
                    write_clock_event(&mut sis_bag, clock, ClockTransition::Rise, rise, &mut done);
                }
            }

            if !done.contains(&(clock.name.clone(), ClockTransition::Fall)) {
                if let Some(fall) = &clock.fall {
                    write_clock_event(&mut sis_bag, clock, ClockTransition::Fall, fall, &mut done);
                }
            }
        }

        self.contents.bags.push(sis_bag);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitState {
    Active,
    Done,
}

struct ResolvedMaster {
    cell: String,
    view: String,
    input_pins: Vec<String>,
    output_pins: Vec<String>,
    is_generic: bool,
}

fn require_node(nodes: &HashMap<String, &SisNode>, name: &str) -> Result<(), OctWriteError> {
    if nodes.contains_key(name) {
        Ok(())
    } else {
        Err(OctWriteError::MissingNode {
            node: name.to_string(),
        })
    }
}

fn attach_policy_properties(facet: &mut OctFacet, options: &OctWriteOptions) {
    facet.properties.insert(
        "TECHNOLOGY".to_string(),
        OctValue::String(options.technology.clone()),
    );
    facet.properties.insert(
        "EDITSTYLE".to_string(),
        OctValue::String(options.editstyle.clone()),
    );
    facet.properties.insert(
        "VIEWTYPE".to_string(),
        OctValue::String(options.viewtype.clone()),
    );
}

fn attach_term_properties(term: &mut OctTerminal, direction: &str, termtype: &str) {
    term.properties.insert(
        "DIRECTION".to_string(),
        OctValue::String(direction.to_string()),
    );
    term.properties.insert(
        "TERMTYPE".to_string(),
        OctValue::String(termtype.to_string()),
    );
}

fn create_interface_from_contents(contents: &OctFacet, options: &OctWriteOptions) -> OctFacet {
    let mut interface = OctFacet::new(&contents.cell, &contents.view, "interface");
    attach_policy_properties(&mut interface, options);
    interface.properties.insert(
        "CELLCLASS".to_string(),
        OctValue::String("MODULE".to_string()),
    );

    for term in &contents.terminals {
        let mut interface_term = OctTerminal::new(&term.name);
        if let Some(direction) = term.properties.get("DIRECTION") {
            interface_term
                .properties
                .insert("DIRECTION".to_string(), direction.clone());
        }
        if let Some(termtype) = term.properties.get("TERMTYPE") {
            interface_term
                .properties
                .insert("TERMTYPE".to_string(), termtype.clone());
        }
        interface.terminals.push(interface_term);
    }

    interface
}

fn write_clock_event(
    sis_bag: &mut OctBag,
    clock: &SisClock,
    transition: ClockTransition,
    event: &ClockEventSpec,
    done: &mut BTreeSet<(String, ClockTransition)>,
) {
    let mut clock_events = OctBag::new("CLOCK_EVENTS");
    clock_events
        .properties
        .insert("TIME".to_string(), OctValue::Real(event.nominal_position));
    write_edge(
        &mut clock_events,
        &clock.name,
        transition,
        event.lower_range,
        event.upper_range,
        done,
    );

    for dependency in &event.dependencies {
        write_edge(
            &mut clock_events,
            &dependency.clock_name,
            dependency.transition,
            dependency.lower_range,
            dependency.upper_range,
            done,
        );
    }

    sis_bag.bags.push(clock_events);
}

fn write_edge(
    clock_events: &mut OctBag,
    clock_name: &str,
    transition: ClockTransition,
    lower_range: OrderedReal,
    upper_range: OrderedReal,
    done: &mut BTreeSet<(String, ClockTransition)>,
) {
    let key = (clock_name.to_string(), transition);
    if !done.insert(key) {
        return;
    }

    let prefix = match transition {
        ClockTransition::Rise => 'r',
        ClockTransition::Fall => 'f',
    };
    let mut event_bag = OctBag::new("EVENT");
    event_bag.properties.insert(
        "DESCRIPTION".to_string(),
        OctValue::String(format!("{prefix}'{clock_name}")),
    );
    event_bag
        .properties
        .insert("MAX".to_string(), OctValue::Real(upper_range));
    event_bag
        .properties
        .insert("MIN".to_string(), OctValue::Real(lower_range));
    clock_events.bags.push(event_bag);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OctWriteError {
    DuplicateNode {
        node: String,
    },
    MissingNode {
        node: String,
    },
    MissingFanin {
        node: String,
        fanin: String,
    },
    CycleDetected {
        node: String,
    },
    PrimaryOutputWithoutFanin {
        node: String,
    },
    TooFewPins {
        node: String,
        pins: usize,
        fanins: usize,
    },
    MissingPin {
        node: String,
        pin_index: usize,
    },
    MissingOutputPin {
        node: String,
    },
    MissingClockPin {
        node: String,
    },
    UndefinedFunction {
        node: String,
    },
    UnknownLiteral {
        fanin: String,
    },
}

impl fmt::Display for OctWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateNode { node } => {
                write!(f, "duplicate SIS node '{node}'")
            }
            Self::MissingNode { node } => {
                write!(f, "missing SIS node '{node}'")
            }
            Self::MissingFanin { node, fanin } => {
                write!(f, "SIS node '{node}' references missing fanin '{fanin}'")
            }
            Self::CycleDetected { node } => {
                write!(f, "SIS network contains a cycle through '{node}'")
            }
            Self::PrimaryOutputWithoutFanin { node } => {
                write!(f, "primary output '{node}' has no fanin")
            }
            Self::TooFewPins { node, pins, fanins } => {
                write!(
                    f,
                    "mapped node '{node}' has {fanins} fanins but only {pins} input pins"
                )
            }
            Self::MissingPin { node, pin_index } => {
                write!(f, "node '{node}' is missing input pin {pin_index}")
            }
            Self::MissingOutputPin { node } => {
                write!(f, "node '{node}' is missing an output pin")
            }
            Self::MissingClockPin { node } => {
                write!(f, "latch node '{node}' is missing a clock pin")
            }
            Self::UndefinedFunction { node } => {
                write!(f, "function for node '{node}' is undefined")
            }
            Self::UnknownLiteral { fanin } => {
                write!(f, "logic function references unknown fanin '{fanin}'")
            }
        }
    }
}

impl Error for OctWriteError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn string_value(value: &str) -> OctValue {
        OctValue::String(value.to_string())
    }

    fn sample_generic_network() -> SisNetwork {
        SisNetwork::new(vec![
            SisNode::primary_input("a"),
            SisNode::primary_input("b"),
            SisNode::internal(
                "n1",
                ["a", "b"],
                NodeFunction::Sop {
                    fanins: vec!["a".to_string(), "b".to_string()],
                    cubes: vec![
                        vec![Literal::positive("a"), Literal::negative("b")],
                        vec![Literal::negative("a"), Literal::positive("b")],
                    ],
                },
            ),
            SisNode::primary_output("out", "n1"),
        ])
    }

    #[test]
    fn unpacks_cell_view_with_default_when_missing() {
        assert_eq!(
            unpack_cell_view("alu:layout", "physical"),
            ("alu".to_string(), "layout".to_string())
        );
        assert_eq!(
            unpack_cell_view("alu", "physical"),
            ("alu".to_string(), "physical".to_string())
        );
    }

    #[test]
    fn writes_policy_terms_nets_interface_and_generic_function() {
        let written = write_oct(
            &sample_generic_network(),
            "top",
            "logic",
            &OctWriteOptions::default(),
        )
        .unwrap();

        assert_eq!(
            written.contents.properties.get("TECHNOLOGY"),
            Some(&string_value("scmos"))
        );
        assert_eq!(
            written.contents.properties.get("CELLCLASS"),
            Some(&string_value("MODULE"))
        );
        assert_eq!(
            written
                .contents
                .terminals
                .iter()
                .map(|term| term.name.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b", "out"]
        );
        assert_eq!(
            written
                .contents
                .nets
                .iter()
                .map(|net| net.name.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b", "n1"]
        );

        let instance = &written.contents.instances[0];
        assert_eq!(instance.master_cell, "top/generic-2");
        assert_eq!(instance.master_view, "physical");
        assert_eq!(
            instance
                .terminals
                .iter()
                .map(|term| (term.name.as_str(), term.net.as_str()))
                .collect::<Vec<_>>(),
            vec![("in0", "a"), ("in1", "b"), ("out0", "n1")]
        );
        assert_eq!(
            instance.terminals[2].properties.get("LOGICFUNCTION"),
            Some(&string_value("(+ (* a (! b)) (* (! a) b))"))
        );

        assert_eq!(written.interface.facet, "interface");
        assert_eq!(
            written
                .interface
                .terminals
                .iter()
                .map(|term| term.name.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b", "out"]
        );
        assert_eq!(written.generic_masters.len(), 1);
        assert_eq!(written.generic_masters[0].input_count, 2);
    }

    #[test]
    fn mapped_gate_uses_configured_cell_path_and_pin_names() {
        let network = SisNetwork::new(vec![
            SisNode::primary_input("a"),
            SisNode::primary_input("b"),
            SisNode::internal(
                "n1",
                ["a", "b"],
                NodeFunction::Factored("(* a b)".to_string()),
            )
            .with_gate(GateBinding::combinational("nand2", ["A", "B"], "Y")),
            SisNode::primary_output("out", "n1"),
        ]);
        let options = OctWriteOptions {
            cell_path: Some("/lib".to_string()),
            ..OctWriteOptions::default()
        };

        let written = write_oct(&network, "top", "logic", &options).unwrap();
        let instance = &written.contents.instances[0];

        assert_eq!(instance.master_cell, "/lib/nand2");
        assert_eq!(
            instance
                .terminals
                .iter()
                .map(|term| (term.name.as_str(), term.net.as_str()))
                .collect::<Vec<_>>(),
            vec![("A", "a"), ("B", "b"), ("Y", "n1")]
        );
        assert!(written.generic_masters.is_empty());
    }

    #[test]
    fn primary_clock_input_marks_net_and_terminal() {
        let network = SisNetwork::new(vec![
            SisNode::primary_input("clk").with_clock_input(true),
            SisNode::primary_output("out", "clk"),
        ]);

        let written = write_oct(&network, "top", "logic", &OctWriteOptions::default()).unwrap();
        let clk_net = written
            .contents
            .nets
            .iter()
            .find(|net| net.name == "clk")
            .unwrap();
        let clk_term = written
            .contents
            .terminals
            .iter()
            .find(|term| term.name == "clk")
            .unwrap();

        assert_eq!(
            clk_net.properties.get("NETTYPE"),
            Some(&string_value("CLOCK"))
        );
        assert_eq!(
            clk_term.properties.get("TERMTYPE"),
            Some(&string_value("CLOCK"))
        );
    }

    #[test]
    fn latch_writes_gate_instance_with_initial_value_and_clock() {
        let network = SisNetwork::new(vec![
            SisNode::primary_input("d"),
            SisNode::primary_input("clk"),
            SisNode::internal("lat_data", ["d"], NodeFunction::Factored("d".to_string())),
            SisNode::primary_output("lat_in", "lat_data").with_real_interface(false),
            SisNode::primary_output("clk_po", "clk").with_real_interface(false),
            SisNode::internal(
                "q",
                ["lat_data"],
                NodeFunction::Factored("lat_data".to_string()),
            ),
            SisNode::primary_output("out", "q"),
        ])
        .with_latches(vec![SisLatch {
            input_output: "lat_in".to_string(),
            output_node: "q".to_string(),
            data_node: "lat_data".to_string(),
            control_output: Some("clk_po".to_string()),
            initial_value: 1,
            gate: GateBinding::combinational("dff", ["D"], "Q")
                .with_clock_pins(["CLK"])
                .with_gate_type(GateType::Sequential),
            feedback_pin: None,
        }]);

        let written = write_oct(&network, "top", "logic", &OctWriteOptions::default()).unwrap();
        let latch = written
            .contents
            .instances
            .iter()
            .find(|instance| instance.master_cell == "dff")
            .unwrap();

        assert_eq!(
            latch
                .terminals
                .iter()
                .map(|term| (term.name.as_str(), term.net.as_str()))
                .collect::<Vec<_>>(),
            vec![("D", "d"), ("Q", "q"), ("CLK", "clk")]
        );
        assert_eq!(
            latch.properties.get("INITIAL_VALUE"),
            Some(&OctValue::Integer(1))
        );
    }

    #[test]
    fn writes_clock_event_bags_once_per_transition() {
        let network = SisNetwork::new(vec![SisNode::primary_input("clk")]).with_clocks(
            vec![SisClock {
                name: "clk".to_string(),
                rise: Some(ClockEventSpec::new(5.0, 4.0, 6.0).with_dependencies(vec![
                    ClockEdgeSpec::new("clk", ClockTransition::Rise, 1.0, 2.0),
                ])),
                fall: Some(ClockEventSpec::new(10.0, 9.0, 11.0)),
            }],
            Some(20.0),
        );

        let written = write_oct(&network, "top", "logic", &OctWriteOptions::default()).unwrap();
        let sis_bag = written
            .contents
            .bags
            .iter()
            .find(|bag| bag.name == "SIS_CLOCKS")
            .unwrap();

        assert_eq!(
            sis_bag.properties.get("CYCLETIME"),
            Some(&OctValue::Real(OrderedReal(20.0)))
        );
        assert_eq!(sis_bag.bags.len(), 2);
        assert_eq!(sis_bag.bags[0].bags.len(), 1);
        assert_eq!(
            sis_bag.bags[0].bags[0].properties.get("DESCRIPTION"),
            Some(&string_value("r'clk"))
        );
        assert_eq!(
            sis_bag.bags[1].bags[0].properties.get("DESCRIPTION"),
            Some(&string_value("f'clk"))
        );
    }

    #[test]
    fn reports_invalid_networks() {
        let duplicate = SisNetwork::new(vec![
            SisNode::primary_input("a"),
            SisNode::primary_input("a"),
        ]);
        assert!(matches!(
            write_oct(&duplicate, "top", "logic", &OctWriteOptions::default()),
            Err(OctWriteError::DuplicateNode { .. })
        ));

        let missing = SisNetwork::new(vec![SisNode::primary_output("out", "missing")]);
        assert!(matches!(
            write_oct(&missing, "top", "logic", &OctWriteOptions::default()),
            Err(OctWriteError::MissingFanin { .. })
        ));

        let undefined = SisNetwork::new(vec![
            SisNode::primary_input("a"),
            SisNode::internal("n1", ["a"], NodeFunction::Undefined),
            SisNode::primary_output("out", "n1"),
        ]);
        assert!(matches!(
            write_oct(&undefined, "top", "logic", &OctWriteOptions::default()),
            Err(OctWriteError::UndefinedFunction { .. })
        ));
    }
}
