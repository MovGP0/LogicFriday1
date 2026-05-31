//! Native Oct logic-view reader.
//!
//! The legacy reader consumed Oct facets directly and built a SIS network from
//! instance terminals, formal terminal logic functions, mapped library gates,
//! clocks, and latch metadata. This port keeps that behavior in an owned Rust
//! model so callers can adapt real Oct data at a higher boundary without adding
//! per-file ABI entry points.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub type OctReadResult<T> = Result<T, OctReadError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellType {
    Combinational,
    Memory,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalDirection {
    Input,
    Output,
    Inout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalType {
    Signal,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Transition {
    Rise,
    Fall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchType {
    ActiveHigh,
    ActiveLow,
    RisingEdge,
    FallingEdge,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SynchModel {
    TransparentLatch,
    MasterSlaveLatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SynchTerm {
    Control,
    ControlBar,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActiveLevel {
    High,
    Low,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoolExpr {
    Constant(bool),
    Literal { name: String, phase: bool },
    And(Vec<BoolExpr>),
    Or(Vec<BoolExpr>),
}

impl BoolExpr {
    pub fn constant(value: bool) -> Self {
        Self::Constant(value)
    }

    pub fn literal(name: impl Into<String>, phase: bool) -> Self {
        Self::Literal {
            name: name.into(),
            phase,
        }
    }

    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Constant(false), _) | (_, Self::Constant(false)) => Self::Constant(false),
            (Self::Constant(true), value) | (value, Self::Constant(true)) => value,
            (Self::And(mut left), Self::And(right)) => {
                left.extend(right);
                Self::And(left)
            }
            (Self::And(mut left), right) => {
                left.push(right);
                Self::And(left)
            }
            (left, Self::And(mut right)) => {
                let mut values = vec![left];
                values.append(&mut right);
                Self::And(values)
            }
            (left, right) => Self::And(vec![left, right]),
        }
    }

    pub fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Constant(true), _) | (_, Self::Constant(true)) => Self::Constant(true),
            (Self::Constant(false), value) | (value, Self::Constant(false)) => value,
            (Self::Or(mut left), Self::Or(right)) => {
                left.extend(right);
                Self::Or(left)
            }
            (Self::Or(mut left), right) => {
                left.push(right);
                Self::Or(left)
            }
            (left, Self::Or(mut right)) => {
                let mut values = vec![left];
                values.append(&mut right);
                Self::Or(values)
            }
            (left, right) => Self::Or(vec![left, right]),
        }
    }

    fn remap(&self, terminals: &BTreeMap<String, String>) -> OctReadResult<Self> {
        match self {
            Self::Constant(value) => Ok(Self::Constant(*value)),
            Self::Literal { name, phase } => {
                let net =
                    terminals
                        .get(name)
                        .ok_or_else(|| OctReadError::MissingTerminalConnection {
                            terminal: name.clone(),
                        })?;

                Ok(Self::Literal {
                    name: net.clone(),
                    phase: *phase,
                })
            }
            Self::And(values) => {
                let mut result = Self::Constant(true);
                for value in values {
                    result = result.and(value.remap(terminals)?);
                }

                Ok(result)
            }
            Self::Or(values) => {
                let mut result = Self::Constant(false);
                for value in values {
                    result = result.or(value.remap(terminals)?);
                }

                Ok(result)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OctNet {
    pub name: String,
    pub net_type: Option<String>,
    pub formal_terminal: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OctFormalTerminal {
    pub name: String,
    pub net: Option<String>,
    pub direction: TerminalDirection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OctInstanceTerminal {
    pub name: String,
    pub net: Option<String>,
    pub logic_function: Option<BoolExpr>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OctMasterTerminal {
    pub name: String,
    pub direction: TerminalDirection,
    pub terminal_type: TerminalType,
    pub logic_function: Option<BoolExpr>,
    pub synch_term: Option<SynchTerm>,
    pub active_level: Option<ActiveLevel>,
}

impl OctMasterTerminal {
    pub fn signal_output(name: impl Into<String>, logic_function: Option<BoolExpr>) -> Self {
        Self {
            name: name.into(),
            direction: TerminalDirection::Output,
            terminal_type: TerminalType::Signal,
            logic_function,
            synch_term: None,
            active_level: None,
        }
    }

    pub fn signal_input(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            direction: TerminalDirection::Input,
            terminal_type: TerminalType::Signal,
            logic_function: None,
            synch_term: None,
            active_level: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OctMaster {
    pub name: String,
    pub view: String,
    pub cell_type: CellType,
    pub synch_model: Option<SynchModel>,
    pub terminals: Vec<OctMasterTerminal>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OctInstance {
    pub name: String,
    pub master: String,
    pub view: String,
    pub terminals: Vec<OctInstanceTerminal>,
    pub initial_value: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockEvent {
    pub description: String,
    pub nominal: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ClockMetadata {
    pub cycle_time: Option<f64>,
    pub events: Vec<ClockEvent>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct OctFacet {
    pub name: String,
    pub nets: Vec<OctNet>,
    pub formal_terminals: Vec<OctFormalTerminal>,
    pub instances: Vec<OctInstance>,
    pub masters: BTreeMap<String, OctMaster>,
    pub clocks: ClockMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryGate {
    pub name: String,
    pub is_combinational: bool,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct Library {
    gates: BTreeMap<String, LibraryGate>,
}

impl Library {
    pub fn add_gate(&mut self, gate: LibraryGate) {
        self.gates.insert(gate.name.clone(), gate);
    }

    pub fn gate(&self, name: &str) -> Option<&LibraryGate> {
        self.gates.get(name)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkNodeKind {
    Unassigned,
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkNode {
    pub name: String,
    pub kind: NetworkNodeKind,
    pub expression: Option<BoolExpr>,
    pub mapped_gate: Option<String>,
    pub mapped_inputs: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Latch {
    pub input: String,
    pub output: String,
    pub control: Option<String>,
    pub latch_type: LatchType,
    pub initial_value: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Clock {
    pub name: String,
    pub node: String,
    pub events: Vec<ClockEdge>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockEdge {
    pub transition: Transition,
    pub nominal: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Network {
    pub name: String,
    pub nodes: BTreeMap<String, NetworkNode>,
    pub primary_inputs: BTreeSet<String>,
    pub primary_outputs: BTreeSet<String>,
    pub latches: Vec<Latch>,
    pub clocks: BTreeMap<String, Clock>,
    pub cycle_time: Option<f64>,
}

impl Network {
    fn ensure_node(&mut self, name: impl Into<String>) -> &mut NetworkNode {
        let name = name.into();
        self.nodes
            .entry(name.clone())
            .or_insert_with(|| NetworkNode {
                name,
                kind: NetworkNodeKind::Unassigned,
                expression: None,
                mapped_gate: None,
                mapped_inputs: Vec::new(),
            })
    }

    fn drive_node(&mut self, name: &str, expression: BoolExpr) -> OctReadResult<()> {
        let node = self.ensure_node(name.to_string());
        if node.expression.is_some() || node.mapped_gate.is_some() {
            return Err(OctReadError::MultiplyDrivenNode {
                node: name.to_string(),
            });
        }

        node.kind = NetworkNodeKind::Internal;
        node.expression = Some(expression);
        Ok(())
    }

    fn map_node(
        &mut self,
        name: &str,
        gate: &LibraryGate,
        mapped_inputs: Vec<String>,
    ) -> OctReadResult<()> {
        {
            let node = self.ensure_node(name.to_string());
            if node.expression.is_some() || node.mapped_gate.is_some() {
                return Err(OctReadError::MultiplyDrivenNode {
                    node: name.to_string(),
                });
            }

            if gate.is_combinational {
                node.kind = NetworkNodeKind::Internal;
            } else {
                node.kind = NetworkNodeKind::PrimaryInput;
            }

            node.mapped_gate = Some(gate.name.clone());
            node.mapped_inputs = mapped_inputs;
        }

        if !gate.is_combinational {
            self.primary_inputs.insert(name.to_string());
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OctReadError {
    EmptyObjectName { object_type: String },
    DuplicateMaster { master: String },
    MissingInstancesBag,
    MissingMaster { master: String },
    MissingTerminal { instance: String, terminal: String },
    MissingTerminalConnection { terminal: String },
    MissingOutputConnection { instance: String, terminal: String },
    MissingLogicFunction { instance: String, terminal: String },
    MissingClockFormalTerminal { net: String },
    MissingClock { name: String },
    InvalidClockDescription { description: String },
    MultiplyDrivenNode { node: String },
    LatchOutputAlreadyDriven { node: String },
}

impl fmt::Display for OctReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyObjectName { object_type } => {
                write!(formatter, "{object_type} has an empty name")
            }
            Self::DuplicateMaster { master } => write!(formatter, "duplicate master {master}"),
            Self::MissingInstancesBag => {
                write!(formatter, "facet does not contain an instances bag")
            }
            Self::MissingMaster { master } => write!(formatter, "missing master {master}"),
            Self::MissingTerminal { instance, terminal } => {
                write!(formatter, "instance {instance} has no terminal {terminal}")
            }
            Self::MissingTerminalConnection { terminal } => {
                write!(formatter, "terminal {terminal} is not connected to a net")
            }
            Self::MissingOutputConnection { instance, terminal } => {
                write!(
                    formatter,
                    "output terminal {terminal} on {instance} is not connected"
                )
            }
            Self::MissingLogicFunction { instance, terminal } => {
                write!(
                    formatter,
                    "output terminal {terminal} on {instance} has no logic function"
                )
            }
            Self::MissingClockFormalTerminal { net } => {
                write!(formatter, "clock net {net} has no formal terminal")
            }
            Self::MissingClock { name } => {
                write!(formatter, "clock {name} is not in the clock list")
            }
            Self::InvalidClockDescription { description } => {
                write!(formatter, "invalid clock event description {description}")
            }
            Self::MultiplyDrivenNode { node } => write!(formatter, "two outputs drive node {node}"),
            Self::LatchOutputAlreadyDriven { node } => {
                write!(formatter, "latch output drives already driven node {node}")
            }
        }
    }
}

impl Error for OctReadError {}

pub fn read_oct(
    facet: &OctFacet,
    mapped: bool,
    library: Option<&Library>,
) -> OctReadResult<Network> {
    if facet.instances.is_empty() {
        return Err(OctReadError::MissingInstancesBag);
    }

    let names = UniqueNames::from_facet(facet)?;
    let mut network = Network {
        name: facet.name.clone(),
        cycle_time: facet.clocks.cycle_time,
        ..Network::default()
    };

    read_clocks(facet, &names, &mut network)?;

    for instance in &facet.instances {
        if mapped {
            if let Some(gate) = find_library_gate(instance, library) {
                read_mapped_gate(instance, gate, &names, &mut network)?;
                continue;
            }
        }

        let master =
            facet
                .masters
                .get(&instance.master)
                .ok_or_else(|| OctReadError::MissingMaster {
                    master: instance.master.clone(),
                })?;

        match master.cell_type {
            CellType::Combinational => read_logic_instance(instance, master, &names, &mut network)?,
            CellType::Memory => read_latch_instance(instance, master, &names, &mut network)?,
            CellType::Other => {}
        }
    }

    read_formal_terminals(facet, &names, &mut network)?;

    Ok(network)
}

fn read_mapped_gate(
    instance: &OctInstance,
    gate: &LibraryGate,
    names: &UniqueNames,
    network: &mut Network,
) -> OctReadResult<()> {
    let terminals = instance_terminal_map(instance, names)?;
    let mut mapped_inputs = Vec::with_capacity(gate.inputs.len());
    for input in &gate.inputs {
        let net = terminals
            .get(input)
            .ok_or_else(|| OctReadError::MissingTerminal {
                instance: instance.name.clone(),
                terminal: input.clone(),
            })?;
        network.ensure_node(net.clone());
        mapped_inputs.push(net.clone());
    }

    for output in &gate.outputs {
        let net = terminals
            .get(output)
            .ok_or_else(|| OctReadError::MissingTerminal {
                instance: instance.name.clone(),
                terminal: output.clone(),
            })?;
        network.map_node(net, gate, mapped_inputs.clone())?;
    }

    Ok(())
}

fn read_logic_instance(
    instance: &OctInstance,
    master: &OctMaster,
    names: &UniqueNames,
    network: &mut Network,
) -> OctReadResult<()> {
    let terminals = instance_terminal_map(instance, names)?;
    for terminal in master.terminals.iter().filter(|terminal| {
        terminal.terminal_type == TerminalType::Signal
            && terminal.direction == TerminalDirection::Output
    }) {
        let actual = find_instance_terminal(instance, &terminal.name)?;
        let logic_function = actual
            .logic_function
            .as_ref()
            .or(terminal.logic_function.as_ref())
            .ok_or_else(|| OctReadError::MissingLogicFunction {
                instance: instance.name.clone(),
                terminal: terminal.name.clone(),
            })?;
        let output_net =
            actual
                .net
                .as_ref()
                .ok_or_else(|| OctReadError::MissingOutputConnection {
                    instance: instance.name.clone(),
                    terminal: terminal.name.clone(),
                })?;
        let output_name = names.net_name(output_net);
        let expression = logic_function.remap(&terminals)?;
        network.drive_node(&output_name, expression)?;
    }

    Ok(())
}

fn read_latch_instance(
    instance: &OctInstance,
    master: &OctMaster,
    names: &UniqueNames,
    network: &mut Network,
) -> OctReadResult<()> {
    let terminals = instance_terminal_map(instance, names)?;
    let output_terminal = master
        .terminals
        .iter()
        .find(|terminal| {
            terminal.terminal_type == TerminalType::Signal
                && terminal.direction == TerminalDirection::Output
        })
        .ok_or_else(|| OctReadError::MissingLogicFunction {
            instance: instance.name.clone(),
            terminal: String::new(),
        })?;

    let actual_output = find_instance_terminal(instance, &output_terminal.name)?;
    let output_net =
        actual_output
            .net
            .as_ref()
            .ok_or_else(|| OctReadError::MissingOutputConnection {
                instance: instance.name.clone(),
                terminal: output_terminal.name.clone(),
            })?;
    let output_name = names.net_name(output_net);

    {
        let output = network.ensure_node(output_name.clone());
        if output.expression.is_some() || output.mapped_gate.is_some() {
            return Err(OctReadError::LatchOutputAlreadyDriven { node: output_name });
        }

        output.kind = NetworkNodeKind::PrimaryInput;
    }
    network.primary_inputs.insert(output_name.clone());

    let function = actual_output
        .logic_function
        .as_ref()
        .or(output_terminal.logic_function.as_ref())
        .ok_or_else(|| OctReadError::MissingLogicFunction {
            instance: instance.name.clone(),
            terminal: output_terminal.name.clone(),
        })?;
    let input_expression = function.remap(&terminals)?;
    let input_name = format!("{}$latch_input", instance.name);
    network.drive_node(&input_name, input_expression)?;

    let control = latch_control(instance, master, &terminals);
    network.latches.push(Latch {
        input: input_name,
        output: output_name,
        control,
        latch_type: latch_type(master),
        initial_value: instance.initial_value.unwrap_or(3),
    });

    Ok(())
}

fn read_formal_terminals(
    facet: &OctFacet,
    names: &UniqueNames,
    network: &mut Network,
) -> OctReadResult<()> {
    for terminal in &facet.formal_terminals {
        let net = terminal
            .net
            .as_ref()
            .ok_or_else(|| OctReadError::MissingTerminalConnection {
                terminal: terminal.name.clone(),
            })?;
        let net_name = names.net_name(net);
        let node = network.ensure_node(net_name.clone());

        match terminal.direction {
            TerminalDirection::Input => {
                node.kind = NetworkNodeKind::PrimaryInput;
                node.name = terminal.name.clone();
                network.primary_inputs.insert(terminal.name.clone());
            }
            TerminalDirection::Output => {
                network.primary_outputs.insert(terminal.name.clone());
                let output_name = terminal.name.clone();
                network.nodes.insert(
                    output_name.clone(),
                    NetworkNode {
                        name: output_name,
                        kind: NetworkNodeKind::PrimaryOutput,
                        expression: Some(BoolExpr::literal(net_name, true)),
                        mapped_gate: None,
                        mapped_inputs: Vec::new(),
                    },
                );
            }
            TerminalDirection::Inout => {}
        }
    }

    Ok(())
}

fn read_clocks(facet: &OctFacet, names: &UniqueNames, network: &mut Network) -> OctReadResult<()> {
    for net in facet
        .nets
        .iter()
        .filter(|net| net.net_type.as_deref() == Some("CLOCK"))
    {
        let formal = net.formal_terminal.as_ref().ok_or_else(|| {
            OctReadError::MissingClockFormalTerminal {
                net: net.name.clone(),
            }
        })?;
        let node_name = names.net_name(&net.name);
        network.ensure_node(node_name.clone());
        network.clocks.insert(
            formal.clone(),
            Clock {
                name: formal.clone(),
                node: node_name,
                events: Vec::new(),
            },
        );
    }

    for event in &facet.clocks.events {
        let (transition, clock_name) = parse_clock_description(&event.description)?;
        let clock =
            network
                .clocks
                .get_mut(clock_name)
                .ok_or_else(|| OctReadError::MissingClock {
                    name: clock_name.to_string(),
                })?;
        clock.events.push(ClockEdge {
            transition,
            nominal: event.nominal,
            min: event.min,
            max: event.max,
        });
    }

    Ok(())
}

fn parse_clock_description(description: &str) -> OctReadResult<(Transition, &str)> {
    let mut chars = description.chars();
    let Some(prefix) = chars.next() else {
        return Err(OctReadError::InvalidClockDescription {
            description: description.to_string(),
        });
    };

    let rest = chars.as_str();
    let clock_name = rest.strip_prefix('\'').unwrap_or(rest);
    if clock_name.is_empty() {
        return Err(OctReadError::InvalidClockDescription {
            description: description.to_string(),
        });
    }

    match prefix {
        'r' => Ok((Transition::Rise, clock_name)),
        'f' => Ok((Transition::Fall, clock_name)),
        _ => Err(OctReadError::InvalidClockDescription {
            description: description.to_string(),
        }),
    }
}

fn find_library_gate<'a>(
    instance: &OctInstance,
    library: Option<&'a Library>,
) -> Option<&'a LibraryGate> {
    let library = library?;
    let master_name = instance
        .master
        .rsplit_once('/')
        .map_or(instance.master.as_str(), |(_, name)| name);
    let gate_name = format!("{}:{}", master_name, instance.view);
    library.gate(&gate_name)
}

fn instance_terminal_map(
    instance: &OctInstance,
    names: &UniqueNames,
) -> OctReadResult<BTreeMap<String, String>> {
    let mut terminals = BTreeMap::new();
    for terminal in &instance.terminals {
        let net = terminal
            .net
            .as_ref()
            .ok_or_else(|| OctReadError::MissingTerminalConnection {
                terminal: terminal.name.clone(),
            })?;
        terminals.insert(terminal.name.clone(), names.net_name(net));
    }

    Ok(terminals)
}

fn find_instance_terminal<'a>(
    instance: &'a OctInstance,
    name: &str,
) -> OctReadResult<&'a OctInstanceTerminal> {
    instance
        .terminals
        .iter()
        .find(|terminal| terminal.name == name)
        .ok_or_else(|| OctReadError::MissingTerminal {
            instance: instance.name.clone(),
            terminal: name.to_string(),
        })
}

fn latch_control(
    instance: &OctInstance,
    master: &OctMaster,
    terminals: &BTreeMap<String, String>,
) -> Option<String> {
    master.terminals.iter().find_map(|terminal| {
        let synch = terminal.synch_term?;
        if !matches!(synch, SynchTerm::Control | SynchTerm::ControlBar) {
            return None;
        }

        if !instance
            .terminals
            .iter()
            .any(|actual| actual.name == terminal.name)
        {
            return None;
        }

        terminals.get(&terminal.name).cloned()
    })
}

fn latch_type(master: &OctMaster) -> LatchType {
    let active_high = master.terminals.iter().find_map(|terminal| {
        let synch = terminal.synch_term?;
        if !matches!(synch, SynchTerm::Control | SynchTerm::ControlBar) {
            return None;
        }

        match terminal.active_level {
            Some(ActiveLevel::Low) => Some(false),
            Some(ActiveLevel::High) | None => Some(true),
        }
    });

    match (master.synch_model, active_high) {
        (Some(SynchModel::TransparentLatch), Some(true)) => LatchType::RisingEdge,
        (Some(SynchModel::TransparentLatch), Some(false)) => LatchType::FallingEdge,
        (Some(SynchModel::MasterSlaveLatch), Some(true)) => LatchType::ActiveHigh,
        (Some(SynchModel::MasterSlaveLatch), Some(false)) => LatchType::ActiveLow,
        _ => LatchType::Unknown,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct UniqueNames {
    nets: BTreeMap<String, String>,
}

impl UniqueNames {
    fn from_facet(facet: &OctFacet) -> OctReadResult<Self> {
        let mut used = BTreeSet::new();
        let mut nets = BTreeMap::new();
        let mut unnamed_count = 0_usize;

        for net in &facet.nets {
            if net.name.is_empty() {
                return Err(OctReadError::EmptyObjectName {
                    object_type: "net".to_string(),
                });
            }

            let mut candidate = net.name.clone();
            if candidate.is_empty() {
                candidate = format!("net-{unnamed_count}");
                unnamed_count += 1;
            }

            if used.contains(&candidate) {
                let base = candidate.clone();
                let mut count = 2;
                loop {
                    candidate = format!("{base}-{{{count}}}");
                    count += 1;
                    if !used.contains(&candidate) {
                        break;
                    }
                }
            }

            used.insert(candidate.clone());
            nets.insert(net.name.clone(), candidate);
        }

        for terminal in &facet.formal_terminals {
            if let Some(net) = &terminal.net {
                if terminal.name == *net {
                    let current = nets.get(net).cloned().unwrap_or_else(|| net.clone());
                    let mut count = 0;
                    loop {
                        let candidate = format!("{current}-{count}");
                        count += 1;
                        if !used.contains(&candidate) {
                            used.insert(candidate.clone());
                            nets.insert(net.clone(), candidate);
                            break;
                        }
                    }
                }
            }
        }

        Ok(Self { nets })
    }

    fn net_name(&self, name: &str) -> String {
        self.nets
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn and2_master() -> OctMaster {
        OctMaster {
            name: "and2".to_string(),
            view: "interface".to_string(),
            cell_type: CellType::Combinational,
            synch_model: None,
            terminals: vec![
                OctMasterTerminal::signal_input("a"),
                OctMasterTerminal::signal_input("b"),
                OctMasterTerminal::signal_output(
                    "y",
                    Some(BoolExpr::And(vec![
                        BoolExpr::literal("a", true),
                        BoolExpr::literal("b", true),
                    ])),
                ),
            ],
        }
    }

    #[test]
    fn reads_combinational_logic_from_master_function() {
        let mut masters = BTreeMap::new();
        masters.insert("and2".to_string(), and2_master());
        let facet = OctFacet {
            name: "logic:contents".to_string(),
            nets: vec![
                OctNet {
                    name: "in_a".to_string(),
                    net_type: None,
                    formal_terminal: None,
                },
                OctNet {
                    name: "in_b".to_string(),
                    net_type: None,
                    formal_terminal: None,
                },
                OctNet {
                    name: "out".to_string(),
                    net_type: None,
                    formal_terminal: None,
                },
            ],
            formal_terminals: vec![
                OctFormalTerminal {
                    name: "a".to_string(),
                    net: Some("in_a".to_string()),
                    direction: TerminalDirection::Input,
                },
                OctFormalTerminal {
                    name: "b".to_string(),
                    net: Some("in_b".to_string()),
                    direction: TerminalDirection::Input,
                },
                OctFormalTerminal {
                    name: "y".to_string(),
                    net: Some("out".to_string()),
                    direction: TerminalDirection::Output,
                },
            ],
            instances: vec![OctInstance {
                name: "u1".to_string(),
                master: "and2".to_string(),
                view: "logic".to_string(),
                terminals: vec![
                    OctInstanceTerminal {
                        name: "a".to_string(),
                        net: Some("in_a".to_string()),
                        logic_function: None,
                    },
                    OctInstanceTerminal {
                        name: "b".to_string(),
                        net: Some("in_b".to_string()),
                        logic_function: None,
                    },
                    OctInstanceTerminal {
                        name: "y".to_string(),
                        net: Some("out".to_string()),
                        logic_function: None,
                    },
                ],
                initial_value: None,
            }],
            masters,
            clocks: ClockMetadata::default(),
        };

        let network = read_oct(&facet, false, None).unwrap();

        assert!(network.primary_inputs.contains("a"));
        assert!(network.primary_inputs.contains("b"));
        assert!(network.primary_outputs.contains("y"));
        assert_eq!(
            network.nodes["out"].expression,
            Some(BoolExpr::And(vec![
                BoolExpr::literal("in_a", true),
                BoolExpr::literal("in_b", true),
            ]))
        );
    }

    #[test]
    fn instance_logic_function_overrides_master_function() {
        let mut masters = BTreeMap::new();
        masters.insert("and2".to_string(), and2_master());
        let facet = OctFacet {
            name: "logic:contents".to_string(),
            nets: vec![
                OctNet {
                    name: "a".to_string(),
                    net_type: None,
                    formal_terminal: None,
                },
                OctNet {
                    name: "b".to_string(),
                    net_type: None,
                    formal_terminal: None,
                },
                OctNet {
                    name: "y".to_string(),
                    net_type: None,
                    formal_terminal: None,
                },
            ],
            formal_terminals: Vec::new(),
            instances: vec![OctInstance {
                name: "u1".to_string(),
                master: "and2".to_string(),
                view: "logic".to_string(),
                terminals: vec![
                    OctInstanceTerminal {
                        name: "a".to_string(),
                        net: Some("a".to_string()),
                        logic_function: None,
                    },
                    OctInstanceTerminal {
                        name: "b".to_string(),
                        net: Some("b".to_string()),
                        logic_function: None,
                    },
                    OctInstanceTerminal {
                        name: "y".to_string(),
                        net: Some("y".to_string()),
                        logic_function: Some(BoolExpr::literal("a", false)),
                    },
                ],
                initial_value: None,
            }],
            masters,
            clocks: ClockMetadata::default(),
        };

        let network = read_oct(&facet, false, None).unwrap();

        assert_eq!(
            network.nodes["y"].expression,
            Some(BoolExpr::literal("a", false))
        );
    }

    #[test]
    fn mapped_gate_is_preserved_when_library_matches() {
        let facet = OctFacet {
            name: "logic:contents".to_string(),
            nets: vec![
                OctNet {
                    name: "a".to_string(),
                    net_type: None,
                    formal_terminal: None,
                },
                OctNet {
                    name: "y".to_string(),
                    net_type: None,
                    formal_terminal: None,
                },
            ],
            formal_terminals: Vec::new(),
            instances: vec![OctInstance {
                name: "u1".to_string(),
                master: "lib/inv".to_string(),
                view: "logic".to_string(),
                terminals: vec![
                    OctInstanceTerminal {
                        name: "a".to_string(),
                        net: Some("a".to_string()),
                        logic_function: None,
                    },
                    OctInstanceTerminal {
                        name: "y".to_string(),
                        net: Some("y".to_string()),
                        logic_function: None,
                    },
                ],
                initial_value: None,
            }],
            masters: BTreeMap::new(),
            clocks: ClockMetadata::default(),
        };
        let mut library = Library::default();
        library.add_gate(LibraryGate {
            name: "inv:logic".to_string(),
            is_combinational: true,
            inputs: vec!["a".to_string()],
            outputs: vec!["y".to_string()],
        });

        let network = read_oct(&facet, true, Some(&library)).unwrap();

        assert_eq!(
            network.nodes["y"].mapped_gate,
            Some("inv:logic".to_string())
        );
        assert_eq!(network.nodes["y"].mapped_inputs, vec!["a"]);
    }

    #[test]
    fn reads_clock_events_from_clock_nets() {
        let mut masters = BTreeMap::new();
        masters.insert("and2".to_string(), and2_master());
        let facet = OctFacet {
            name: "logic:contents".to_string(),
            nets: vec![OctNet {
                name: "clk_net".to_string(),
                net_type: Some("CLOCK".to_string()),
                formal_terminal: Some("clk".to_string()),
            }],
            formal_terminals: Vec::new(),
            instances: vec![OctInstance {
                name: "dummy".to_string(),
                master: "and2".to_string(),
                view: "logic".to_string(),
                terminals: vec![
                    OctInstanceTerminal {
                        name: "a".to_string(),
                        net: Some("clk_net".to_string()),
                        logic_function: None,
                    },
                    OctInstanceTerminal {
                        name: "b".to_string(),
                        net: Some("clk_net".to_string()),
                        logic_function: None,
                    },
                    OctInstanceTerminal {
                        name: "y".to_string(),
                        net: Some("clk_net".to_string()),
                        logic_function: None,
                    },
                ],
                initial_value: None,
            }],
            masters,
            clocks: ClockMetadata {
                cycle_time: Some(10.0),
                events: vec![ClockEvent {
                    description: "r'clk".to_string(),
                    nominal: Some(2.0),
                    min: Some(1.0),
                    max: Some(3.0),
                }],
            },
        };

        let network = read_oct(&facet, false, None).unwrap();

        assert_eq!(network.cycle_time, Some(10.0));
        assert_eq!(network.clocks["clk"].events[0].transition, Transition::Rise);
        assert_eq!(network.clocks["clk"].events[0].nominal, Some(2.0));
    }

    #[test]
    fn reads_memory_cell_as_latch() {
        let mut control = OctMasterTerminal::signal_input("clk");
        control.synch_term = Some(SynchTerm::Control);
        control.active_level = Some(ActiveLevel::High);
        let master = OctMaster {
            name: "dff".to_string(),
            view: "interface".to_string(),
            cell_type: CellType::Memory,
            synch_model: Some(SynchModel::MasterSlaveLatch),
            terminals: vec![
                OctMasterTerminal::signal_input("d"),
                control,
                OctMasterTerminal::signal_output("q", Some(BoolExpr::literal("d", true))),
            ],
        };
        let mut masters = BTreeMap::new();
        masters.insert("dff".to_string(), master);
        let facet = OctFacet {
            name: "logic:contents".to_string(),
            nets: vec![
                OctNet {
                    name: "d".to_string(),
                    net_type: None,
                    formal_terminal: None,
                },
                OctNet {
                    name: "clk".to_string(),
                    net_type: None,
                    formal_terminal: None,
                },
                OctNet {
                    name: "q".to_string(),
                    net_type: None,
                    formal_terminal: None,
                },
            ],
            formal_terminals: Vec::new(),
            instances: vec![OctInstance {
                name: "ff0".to_string(),
                master: "dff".to_string(),
                view: "logic".to_string(),
                terminals: vec![
                    OctInstanceTerminal {
                        name: "d".to_string(),
                        net: Some("d".to_string()),
                        logic_function: None,
                    },
                    OctInstanceTerminal {
                        name: "clk".to_string(),
                        net: Some("clk".to_string()),
                        logic_function: None,
                    },
                    OctInstanceTerminal {
                        name: "q".to_string(),
                        net: Some("q".to_string()),
                        logic_function: None,
                    },
                ],
                initial_value: Some(1),
            }],
            masters,
            clocks: ClockMetadata::default(),
        };

        let network = read_oct(&facet, false, None).unwrap();

        assert_eq!(network.latches.len(), 1);
        assert_eq!(network.latches[0].output, "q");
        assert_eq!(network.latches[0].control, Some("clk".to_string()));
        assert_eq!(network.latches[0].latch_type, LatchType::ActiveHigh);
        assert_eq!(network.latches[0].initial_value, 1);
    }

    #[test]
    fn duplicate_outputs_are_rejected() {
        let mut masters = BTreeMap::new();
        masters.insert("and2".to_string(), and2_master());
        let instance = OctInstance {
            name: "u1".to_string(),
            master: "and2".to_string(),
            view: "logic".to_string(),
            terminals: vec![
                OctInstanceTerminal {
                    name: "a".to_string(),
                    net: Some("a".to_string()),
                    logic_function: None,
                },
                OctInstanceTerminal {
                    name: "b".to_string(),
                    net: Some("b".to_string()),
                    logic_function: None,
                },
                OctInstanceTerminal {
                    name: "y".to_string(),
                    net: Some("y".to_string()),
                    logic_function: None,
                },
            ],
            initial_value: None,
        };
        let facet = OctFacet {
            name: "logic:contents".to_string(),
            nets: vec![
                OctNet {
                    name: "a".to_string(),
                    net_type: None,
                    formal_terminal: None,
                },
                OctNet {
                    name: "b".to_string(),
                    net_type: None,
                    formal_terminal: None,
                },
                OctNet {
                    name: "y".to_string(),
                    net_type: None,
                    formal_terminal: None,
                },
            ],
            formal_terminals: Vec::new(),
            instances: vec![instance.clone(), instance],
            masters,
            clocks: ClockMetadata::default(),
        };

        let error = read_oct(&facet, false, None).unwrap_err();

        assert_eq!(
            error,
            OctReadError::MultiplyDrivenNode {
                node: "y".to_string()
            }
        );
    }

    #[test]
    fn no_legacy_abi_tokens_are_present() {
        let source = include_str!("octread.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
