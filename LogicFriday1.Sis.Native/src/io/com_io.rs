//! Native command model for `LogicSynthesis/sis/io/com_io.c`.
//!
//! The legacy file is mostly a command dispatcher around SIS IO readers,
//! writers, printers, and a few local network mutations. This port keeps the
//! command registration, option parsing, PDS formatting, IO/stat printing,
//! name-mode handling, and IO polarity operations in safe Rust. File parsing
//! and writer integrations are expressed through backend traits so higher
//! integration layers can bind the already-ported reader/writer modules without
//! adding per-file C ABI entry points.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IoNodeId(usize);

impl IoNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IoNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IoLiteral {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IoCube {
    pub literals: Vec<IoLiteral>,
}

impl IoCube {
    pub fn new(literals: impl Into<Vec<IoLiteral>>) -> Self {
        Self {
            literals: literals.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IoFunction {
    Zero,
    One,
    Sop(Vec<IoCube>),
    Buffer,
    Inverter,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IoNode {
    pub name: String,
    pub short_name: String,
    pub kind: IoNodeKind,
    pub fanins: Vec<IoNodeId>,
    pub fanouts: BTreeSet<IoNodeId>,
    pub function: IoFunction,
    pub is_real_pi: bool,
    pub is_real_po: bool,
}

impl IoNode {
    pub fn new(name: impl Into<String>, kind: IoNodeKind) -> Self {
        let name = name.into();
        Self {
            short_name: name.clone(),
            name,
            kind,
            fanins: Vec::new(),
            fanouts: BTreeSet::new(),
            function: IoFunction::Sop(Vec::new()),
            is_real_pi: kind == IoNodeKind::PrimaryInput,
            is_real_po: kind == IoNodeKind::PrimaryOutput,
        }
    }

    pub fn with_short_name(mut self, short_name: impl Into<String>) -> Self {
        self.short_name = short_name.into();
        self
    }

    pub fn with_fanins(mut self, fanins: impl Into<Vec<IoNodeId>>) -> Self {
        self.fanins = fanins.into();
        self
    }

    pub fn with_function(mut self, function: IoFunction) -> Self {
        self.function = function;
        self
    }

    pub fn virtual_io(mut self) -> Self {
        self.is_real_pi = false;
        self.is_real_po = false;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchInitialValue {
    Zero,
    One,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IoLatch {
    pub input: IoNodeId,
    pub output: IoNodeId,
    pub initial_value: LatchInitialValue,
}

impl IoLatch {
    pub fn new(input: IoNodeId, output: IoNodeId, initial_value: LatchInitialValue) -> Self {
        Self {
            input,
            output,
            initial_value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IoNetwork {
    name: String,
    nodes: Vec<Option<IoNode>>,
    order: Vec<IoNodeId>,
    primary_inputs: Vec<IoNodeId>,
    primary_outputs: Vec<IoNodeId>,
    name_table: BTreeMap<String, IoNodeId>,
    latches: Vec<IoLatch>,
    dc_network: Option<Box<IoNetwork>>,
    next_generated: usize,
}

impl Default for IoNetwork {
    fn default() -> Self {
        Self::new("unknown")
    }
}

impl IoNetwork {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            order: Vec::new(),
            primary_inputs: Vec::new(),
            primary_outputs: Vec::new(),
            name_table: BTreeMap::new(),
            latches: Vec::new(),
            dc_network: None,
            next_generated: 0,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn add_node(&mut self, node: IoNode) -> Result<IoNodeId, IoCommandError> {
        if self.name_table.contains_key(&node.name) {
            return Err(IoCommandError::DuplicateNode(node.name));
        }

        for fanin in &node.fanins {
            self.node(*fanin)?;
        }

        let id = IoNodeId(self.nodes.len());
        if node.kind == IoNodeKind::PrimaryInput {
            self.primary_inputs.push(id);
        }
        if node.kind == IoNodeKind::PrimaryOutput {
            self.primary_outputs.push(id);
        }

        let fanins = node.fanins.clone();
        self.name_table.insert(node.name.clone(), id);
        self.nodes.push(Some(node));
        self.order.push(id);

        for fanin in fanins {
            self.node_mut(fanin)?.fanouts.insert(id);
        }

        Ok(id)
    }

    pub fn add_primary_input(
        &mut self,
        name: impl Into<String>,
    ) -> Result<IoNodeId, IoCommandError> {
        self.add_node(IoNode::new(name, IoNodeKind::PrimaryInput).with_function(IoFunction::Buffer))
    }

    pub fn add_internal(
        &mut self,
        name: impl Into<String>,
        fanins: impl Into<Vec<IoNodeId>>,
        function: IoFunction,
    ) -> Result<IoNodeId, IoCommandError> {
        self.add_node(
            IoNode::new(name, IoNodeKind::Internal)
                .with_fanins(fanins)
                .with_function(function),
        )
    }

    pub fn add_primary_output(
        &mut self,
        name: impl Into<String>,
        fanin: IoNodeId,
    ) -> Result<IoNodeId, IoCommandError> {
        self.add_node(
            IoNode::new(name, IoNodeKind::PrimaryOutput)
                .with_fanins([fanin])
                .with_function(IoFunction::Buffer),
        )
    }

    pub fn add_latch(&mut self, latch: IoLatch) -> Result<(), IoCommandError> {
        self.node(latch.input)?;
        self.node(latch.output)?;
        self.latches.push(latch);
        Ok(())
    }

    pub fn latches(&self) -> &[IoLatch] {
        &self.latches
    }

    pub fn node(&self, node: IoNodeId) -> Result<&IoNode, IoCommandError> {
        self.nodes
            .get(node.index())
            .and_then(Option::as_ref)
            .ok_or(IoCommandError::MissingNodeById(node))
    }

    pub fn node_mut(&mut self, node: IoNodeId) -> Result<&mut IoNode, IoCommandError> {
        self.nodes
            .get_mut(node.index())
            .and_then(Option::as_mut)
            .ok_or(IoCommandError::MissingNodeById(node))
    }

    pub fn find_node(&self, name: &str) -> Option<IoNodeId> {
        self.name_table.get(name).copied()
    }

    pub fn nodes(&self) -> impl Iterator<Item = (IoNodeId, &IoNode)> {
        self.order.iter().filter_map(|id| {
            self.nodes
                .get(id.index())
                .and_then(Option::as_ref)
                .map(|node| (*id, node))
        })
    }

    pub fn primary_inputs(&self) -> &[IoNodeId] {
        &self.primary_inputs
    }

    pub fn primary_outputs(&self) -> &[IoNodeId] {
        &self.primary_outputs
    }

    pub fn dc_network(&self) -> Option<&IoNetwork> {
        self.dc_network.as_deref()
    }

    pub fn set_dc_network(&mut self, dc_network: Option<IoNetwork>) {
        self.dc_network = dc_network.map(Box::new);
    }

    pub fn reset_short_names(&mut self) {
        for (index, node) in self.nodes.iter_mut().filter_map(Option::as_mut).enumerate() {
            node.short_name = generated_short_name(index);
        }
    }

    pub fn reset_long_names(&mut self) {
        let ids = self.order.clone();
        self.name_table.clear();

        for id in ids {
            if let Some(node) = self.nodes.get_mut(id.index()).and_then(Option::as_mut) {
                if is_madeup_name(&node.name) {
                    node.name = format!("[{}]", id.index());
                }
                self.name_table.insert(node.name.clone(), id);
            }
        }
    }

    pub fn node_name(&self, node: IoNodeId, short_name: bool) -> Result<&str, IoCommandError> {
        let node = self.node(node)?;
        if short_name {
            Ok(&node.short_name)
        } else {
            Ok(&node.name)
        }
    }

    pub fn selected_nodes(&self, names: &[String]) -> Result<Vec<IoNodeId>, IoCommandError> {
        if names.is_empty() {
            return Ok(self.nodes().map(|(id, _)| id).collect());
        }

        names
            .iter()
            .map(|name| {
                self.find_node(name)
                    .ok_or_else(|| IoCommandError::MissingNode(name.clone()))
            })
            .collect()
    }

    pub fn true_io_nodes(&self, names: &[String]) -> Result<Vec<IoNodeId>, IoCommandError> {
        names
            .iter()
            .map(|name| {
                self.find_node(name)
                    .ok_or_else(|| IoCommandError::MissingNode(name.clone()))
            })
            .collect()
    }

    pub fn invert_io_nodes(&mut self, nodes: &[IoNodeId]) -> Result<(), IoCommandError> {
        for node in nodes {
            let kind = self.node(*node)?.kind;
            let is_real_input = self.node(*node)?.is_real_pi;
            let is_real_output = self.node(*node)?.is_real_po;
            if !(kind == IoNodeKind::PrimaryInput && is_real_input
                || kind == IoNodeKind::PrimaryOutput && is_real_output)
            {
                return Err(IoCommandError::NotExternalIo(
                    self.node(*node)?.name.clone(),
                ));
            }
        }

        for node in nodes {
            self.invert_polarity(*node)?;
        }

        self.sweep_unobserved();
        self.dc_network = None;
        Ok(())
    }

    pub fn force_initial_state_to_zero(&mut self) -> Result<(), IoCommandError> {
        let mut nodes_to_invert = Vec::new();
        for latch in &mut self.latches {
            if latch.initial_value == LatchInitialValue::One {
                nodes_to_invert.push(latch.output);
                nodes_to_invert.push(latch.input);
                latch.initial_value = LatchInitialValue::Zero;
            }
        }

        for node in nodes_to_invert {
            self.invert_polarity(node)?;
        }

        self.sweep_unobserved();
        self.dc_network = None;
        Ok(())
    }

    fn invert_polarity(&mut self, node: IoNodeId) -> Result<(), IoCommandError> {
        match self.node(node)?.kind {
            IoNodeKind::PrimaryInput => {
                let fanouts = self.node(node)?.fanouts.iter().copied().collect::<Vec<_>>();
                let inverter = self.add_inverter(node)?;
                for fanout in fanouts {
                    if fanout != inverter {
                        self.patch_fanin(fanout, node, inverter)?;
                    }
                }
            }
            IoNodeKind::PrimaryOutput => {
                let fanin = *self
                    .node(node)?
                    .fanins
                    .first()
                    .ok_or(IoCommandError::InvalidPrimaryOutput(node))?;
                let inverter = self.add_inverter(fanin)?;
                self.patch_fanin(node, fanin, inverter)?;
            }
            IoNodeKind::Internal => {
                return Err(IoCommandError::NotExternalIo(self.node(node)?.name.clone()));
            }
        }

        Ok(())
    }

    fn add_inverter(&mut self, fanin: IoNodeId) -> Result<IoNodeId, IoCommandError> {
        let name = loop {
            let candidate = format!("[inv{}]", self.next_generated);
            self.next_generated += 1;
            if !self.name_table.contains_key(&candidate) {
                break candidate;
            }
        };

        self.add_internal(name, [fanin], IoFunction::Inverter)
    }

    fn patch_fanin(
        &mut self,
        node: IoNodeId,
        old_fanin: IoNodeId,
        new_fanin: IoNodeId,
    ) -> Result<(), IoCommandError> {
        self.node(old_fanin)?;
        self.node(new_fanin)?;
        let mut replaced = false;
        for fanin in &mut self.node_mut(node)?.fanins {
            if *fanin == old_fanin {
                *fanin = new_fanin;
                replaced = true;
            }
        }

        if replaced {
            self.node_mut(old_fanin)?.fanouts.remove(&node);
            self.node_mut(new_fanin)?.fanouts.insert(node);
        }

        Ok(())
    }

    fn sweep_unobserved(&mut self) {
        loop {
            let removable = self
                .nodes()
                .find(|(_, node)| node.kind == IoNodeKind::Internal && node.fanouts.is_empty())
                .map(|(id, _)| id);
            let Some(removable) = removable else {
                break;
            };
            let _ = self.delete_node(removable);
        }
    }

    fn delete_node(&mut self, node: IoNodeId) -> Result<(), IoCommandError> {
        let removed = self.nodes[node.index()]
            .take()
            .ok_or(IoCommandError::MissingNodeById(node))?;
        self.order.retain(|id| *id != node);
        self.name_table.remove(&removed.name);

        for fanin in removed.fanins {
            if let Some(fanin_node) = self.nodes.get_mut(fanin.index()).and_then(Option::as_mut) {
                fanin_node.fanouts.remove(&node);
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IoCommandKind {
    BdSyn,
    ReadBlif,
    ReadPla,
    ReadEqn,
    ReadKiss,
    ReadSlif,
    WriteBdnet,
    WriteBlif,
    WriteEqn,
    WritePla,
    WritePds,
    WriteKiss,
    WriteSlif,
    Print,
    PrintIo,
    PrintStats,
    ChangeName,
    ResetName,
    PrintAltName,
    InvertIo,
    ForceInitZero,
    PlotBlif,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub kind: IoCommandKind,
    pub changes_network: bool,
}

pub const IO_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "bdsyn",
        kind: IoCommandKind::BdSyn,
        changes_network: false,
    },
    CommandRegistration {
        name: "read_blif",
        kind: IoCommandKind::ReadBlif,
        changes_network: true,
    },
    CommandRegistration {
        name: "read_pla",
        kind: IoCommandKind::ReadPla,
        changes_network: true,
    },
    CommandRegistration {
        name: "read_eqn",
        kind: IoCommandKind::ReadEqn,
        changes_network: true,
    },
    CommandRegistration {
        name: "read_kiss",
        kind: IoCommandKind::ReadKiss,
        changes_network: true,
    },
    CommandRegistration {
        name: "read_slif",
        kind: IoCommandKind::ReadSlif,
        changes_network: true,
    },
    CommandRegistration {
        name: "write_bdnet",
        kind: IoCommandKind::WriteBdnet,
        changes_network: false,
    },
    CommandRegistration {
        name: "write_blif",
        kind: IoCommandKind::WriteBlif,
        changes_network: false,
    },
    CommandRegistration {
        name: "write_eqn",
        kind: IoCommandKind::WriteEqn,
        changes_network: false,
    },
    CommandRegistration {
        name: "write_pla",
        kind: IoCommandKind::WritePla,
        changes_network: false,
    },
    CommandRegistration {
        name: "write_pds",
        kind: IoCommandKind::WritePds,
        changes_network: false,
    },
    CommandRegistration {
        name: "write_kiss",
        kind: IoCommandKind::WriteKiss,
        changes_network: false,
    },
    CommandRegistration {
        name: "write_slif",
        kind: IoCommandKind::WriteSlif,
        changes_network: false,
    },
    CommandRegistration {
        name: "print",
        kind: IoCommandKind::Print,
        changes_network: false,
    },
    CommandRegistration {
        name: "print_io",
        kind: IoCommandKind::PrintIo,
        changes_network: false,
    },
    CommandRegistration {
        name: "print_stats",
        kind: IoCommandKind::PrintStats,
        changes_network: false,
    },
    CommandRegistration {
        name: "chng_name",
        kind: IoCommandKind::ChangeName,
        changes_network: false,
    },
    CommandRegistration {
        name: "reset_name",
        kind: IoCommandKind::ResetName,
        changes_network: false,
    },
    CommandRegistration {
        name: "print_altname",
        kind: IoCommandKind::PrintAltName,
        changes_network: false,
    },
    CommandRegistration {
        name: "invert_io",
        kind: IoCommandKind::InvertIo,
        changes_network: true,
    },
    CommandRegistration {
        name: "force_init_0",
        kind: IoCommandKind::ForceInitZero,
        changes_network: true,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameMode {
    Long,
    Short,
}

impl NameMode {
    pub fn toggle(&mut self) -> &'static str {
        match self {
            Self::Long => {
                *self = Self::Short;
                "changing to short-name mode"
            }
            Self::Short => {
                *self = Self::Long;
                "changing to long-name mode"
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IoCommand {
    Read {
        kind: IoCommandKind,
        append: bool,
        single_output: bool,
        filename: String,
    },
    Write {
        kind: IoCommandKind,
        short_name: bool,
        net_list: bool,
        delays: bool,
        filename: String,
    },
    WritePds {
        short_name: bool,
        combinational: bool,
        debug: bool,
        filename: String,
    },
    Print {
        negative: bool,
        print_dc: bool,
        nodes: Vec<String>,
    },
    PrintIo {
        print_dc: bool,
        nodes: Vec<String>,
    },
    PrintStats {
        factored_literals: bool,
        print_dc: bool,
        nodes: Vec<String>,
    },
    ChangeName,
    ResetName {
        short_names: bool,
        long_names: bool,
    },
    PrintAltName {
        nodes: Vec<String>,
    },
    InvertIo {
        nodes: Vec<String>,
    },
    ForceInitZero,
    BdSyn {
        minimize_each_node: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IoCommandOutput {
    pub status: i32,
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
}

impl IoCommandOutput {
    pub fn success() -> Self {
        Self {
            status: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    pub fn with_stdout(stdout: Vec<String>) -> Self {
        Self {
            status: 0,
            stdout,
            stderr: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IoCommandError {
    UnknownCommand(String),
    Usage {
        command: IoCommandKind,
        usage: &'static str,
    },
    UnsupportedOption(String),
    MissingNode(String),
    MissingNodeById(IoNodeId),
    DuplicateNode(String),
    InvalidPrimaryOutput(IoNodeId),
    InvalidCover {
        node: IoNodeId,
        cube: usize,
    },
    NotExternalIo(String),
    MissingNetwork,
    Backend(String),
}

impl fmt::Display for IoCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(command) => write!(formatter, "unknown IO command {command}"),
            Self::Usage { usage, .. } => write!(formatter, "{usage}"),
            Self::UnsupportedOption(option) => write!(formatter, "unsupported option {option}"),
            Self::MissingNode(name) => write!(formatter, "node {name} was not found"),
            Self::MissingNodeById(node) => write!(formatter, "node {} was not found", node.index()),
            Self::DuplicateNode(name) => write!(formatter, "duplicate node {name}"),
            Self::InvalidPrimaryOutput(node) => write!(
                formatter,
                "primary output {} has invalid fanin count",
                node.index()
            ),
            Self::InvalidCover { node, cube } => write!(
                formatter,
                "node {} has invalid cover cube {cube}",
                node.index()
            ),
            Self::NotExternalIo(name) => {
                write!(formatter, "Error: node {name} is not an external PI or PO")
            }
            Self::MissingNetwork => write!(formatter, "current network is missing"),
            Self::Backend(message) => formatter.write_str(message),
        }
    }
}

impl Error for IoCommandError {}

pub trait IoCommandBackend {
    fn read_network(
        &mut self,
        kind: IoCommandKind,
        filename: &str,
        single_output: bool,
    ) -> Result<IoNetwork, IoCommandError>;

    fn write_network(
        &mut self,
        kind: IoCommandKind,
        network: &IoNetwork,
        options: &WriteOptions,
    ) -> Result<Vec<String>, IoCommandError>;

    fn append_network(
        &mut self,
        current: &mut IoNetwork,
        new_network: IoNetwork,
    ) -> Result<(), IoCommandError> {
        let _ = current;
        let _ = new_network;
        Err(IoCommandError::Backend(
            "network append backend is unavailable".to_owned(),
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteOptions {
    pub short_name: bool,
    pub net_list: bool,
    pub delays: bool,
    pub filename: String,
}

pub fn io_command_registrations(graphics_enabled: bool) -> Vec<CommandRegistration> {
    let mut commands = IO_COMMANDS.to_vec();
    if graphics_enabled {
        commands.push(CommandRegistration {
            name: "plot_blif",
            kind: IoCommandKind::PlotBlif,
            changes_network: false,
        });
    }
    commands
}

pub fn parse_io_command<I, S>(command_name: &str, args: I) -> Result<IoCommand, IoCommandError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    match command_name {
        "read_blif" => parse_read_command(IoCommandKind::ReadBlif, &args),
        "read_pla" => parse_read_command(IoCommandKind::ReadPla, &args),
        "read_eqn" => parse_read_command(IoCommandKind::ReadEqn, &args),
        "read_kiss" => parse_read_command(IoCommandKind::ReadKiss, &args),
        "read_slif" => parse_read_command(IoCommandKind::ReadSlif, &args),
        "write_bdnet" => parse_write_command(IoCommandKind::WriteBdnet, &args),
        "write_blif" => parse_write_command(IoCommandKind::WriteBlif, &args),
        "write_eqn" => parse_write_command(IoCommandKind::WriteEqn, &args),
        "write_pla" => parse_write_command(IoCommandKind::WritePla, &args),
        "write_kiss" => parse_write_command(IoCommandKind::WriteKiss, &args),
        "write_slif" => parse_write_command(IoCommandKind::WriteSlif, &args),
        "write_pds" => parse_write_pds_command(&args),
        "print" => parse_print_command(&args),
        "print_io" => parse_print_io_command(&args),
        "print_stats" => parse_print_stats_command(&args),
        "chng_name" => {
            parse_no_arg_command(IoCommandKind::ChangeName, &args).map(|()| IoCommand::ChangeName)
        }
        "reset_name" => parse_reset_name_command(&args),
        "print_altname" => parse_print_altname_command(&args),
        "invert_io" => parse_invert_io_command(&args),
        "force_init_0" => parse_no_arg_command(IoCommandKind::ForceInitZero, &args)
            .map(|()| IoCommand::ForceInitZero),
        "bdsyn" => parse_bdsyn_command(&args),
        _ => Err(IoCommandError::UnknownCommand(command_name.to_owned())),
    }
}

pub fn dispatch_io_command<B>(
    network: &mut Option<IoNetwork>,
    name_mode: &mut NameMode,
    backend: &mut B,
    command: &IoCommand,
) -> Result<IoCommandOutput, IoCommandError>
where
    B: IoCommandBackend,
{
    match command {
        IoCommand::Read {
            kind,
            append,
            single_output,
            filename,
        } => {
            let new_network = backend.read_network(*kind, filename, *single_output)?;
            if *append {
                let current = network.as_mut().ok_or(IoCommandError::MissingNetwork)?;
                backend.append_network(current, new_network)?;
            } else {
                *network = Some(new_network);
            }
            if let Some(network) = network.as_mut() {
                network.reset_short_names();
            }
            Ok(IoCommandOutput::success())
        }
        IoCommand::Write {
            kind,
            short_name,
            net_list,
            delays,
            filename,
        } => {
            let network = network.as_ref().ok_or(IoCommandError::MissingNetwork)?;
            let output = backend.write_network(
                *kind,
                network,
                &WriteOptions {
                    short_name: *short_name,
                    net_list: *net_list,
                    delays: *delays,
                    filename: filename.clone(),
                },
            )?;
            Ok(IoCommandOutput::with_stdout(output))
        }
        IoCommand::WritePds {
            short_name,
            combinational,
            ..
        } => {
            let network = network.as_ref().ok_or(IoCommandError::MissingNetwork)?;
            Ok(IoCommandOutput::with_stdout(vec![write_palasm(
                network,
                *short_name,
                *combinational,
            )?]))
        }
        IoCommand::Print {
            negative,
            print_dc,
            nodes,
        } => {
            let selected = selected_network(network.as_ref(), *print_dc)?;
            print_nodes(selected, *negative, nodes)
        }
        IoCommand::PrintIo { print_dc, nodes } => {
            let selected = selected_network(network.as_ref(), *print_dc)?;
            print_io(selected, nodes)
        }
        IoCommand::PrintStats {
            factored_literals,
            print_dc,
            nodes,
        } => {
            let selected = selected_network(network.as_ref(), *print_dc)?;
            print_stats(selected, *factored_literals, nodes)
        }
        IoCommand::ChangeName => Ok(IoCommandOutput::with_stdout(vec![
            name_mode.toggle().to_owned(),
        ])),
        IoCommand::ResetName {
            short_names,
            long_names,
        } => {
            let network = network.as_mut().ok_or(IoCommandError::MissingNetwork)?;
            if *short_names {
                network.reset_short_names();
            }
            if *long_names {
                network.reset_long_names();
            }
            Ok(IoCommandOutput::success())
        }
        IoCommand::PrintAltName { nodes } => {
            let network = network.as_ref().ok_or(IoCommandError::MissingNetwork)?;
            print_alt_names(network, nodes)
        }
        IoCommand::InvertIo { nodes } => {
            let network = network.as_mut().ok_or(IoCommandError::MissingNetwork)?;
            let node_ids = network.true_io_nodes(nodes)?;
            network.invert_io_nodes(&node_ids)?;
            Ok(IoCommandOutput::success())
        }
        IoCommand::ForceInitZero => {
            let network = network.as_mut().ok_or(IoCommandError::MissingNetwork)?;
            network.force_initial_state_to_zero()?;
            Ok(IoCommandOutput::success())
        }
        IoCommand::BdSyn { .. } => Err(IoCommandError::Backend(
            "bdsyn streaming BLIF backend is unavailable".to_owned(),
        )),
    }
}

pub fn write_palasm(
    network: &IoNetwork,
    short_name: bool,
    combinational: bool,
) -> Result<String, IoCommandError> {
    let mut output = String::new();
    output.push_str("CHIP dummy LCA\n\n;PINLIST\n");
    let mut printed = 0usize;

    for input in network.primary_inputs() {
        let node = network.node(*input)?;
        if !combinational && !node.is_real_pi {
            continue;
        }
        push_pin(
            &mut output,
            network.node_name(*input, short_name)?,
            &mut printed,
        );
    }

    for output_node in network.primary_outputs() {
        let node = network.node(*output_node)?;
        if !combinational && !node.is_real_po {
            continue;
        }
        push_pin(
            &mut output,
            network.node_name(*output_node, short_name)?,
            &mut printed,
        );
    }

    output.push('\n');
    if !combinational && !network.latches().is_empty() {
        output.push_str("clock\n");
    }
    output.push_str("EQUATIONS\n");

    for (node_id, _) in network.nodes() {
        if let Some(line) = write_pds_sop(network, node_id, short_name)? {
            output.push_str(&line);
            output.push('\n');
        }
    }

    if !combinational {
        for latch in network.latches() {
            let input_name = network.node_name(latch.input, short_name)?;
            let output_name = network.node_name(latch.output, short_name)?;
            output.push_str(output_name);
            output.push_str(" := ");
            output.push_str(input_name);
            output.push('\n');
            output.push_str(output_name);
            output.push_str(".CLKF = clock\n");
        }
    }

    Ok(output)
}

pub fn write_pds_sop(
    network: &IoNetwork,
    node_id: IoNodeId,
    short_name: bool,
) -> Result<Option<String>, IoCommandError> {
    let node = network.node(node_id)?;
    if !io_node_should_be_printed(node) {
        return Ok(None);
    }

    let mut line = format!("{} = ", network.node_name(node_id, short_name)?);
    match &node.function {
        IoFunction::Zero => {
            line.push_str("GND");
        }
        IoFunction::One => {
            line.push_str("VCC");
        }
        IoFunction::Buffer if node.kind == IoNodeKind::PrimaryOutput => {
            let fanin = *node
                .fanins
                .first()
                .ok_or(IoCommandError::InvalidPrimaryOutput(node_id))?;
            line.push_str(network.node_name(fanin, short_name)?);
        }
        IoFunction::Inverter => {
            let fanin = *node
                .fanins
                .first()
                .ok_or(IoCommandError::InvalidPrimaryOutput(node_id))?;
            line.push('/');
            line.push_str(network.node_name(fanin, short_name)?);
        }
        IoFunction::Buffer => {
            if let Some(fanin) = node.fanins.first() {
                line.push_str(network.node_name(*fanin, short_name)?);
            } else {
                line.push_str("VCC");
            }
        }
        IoFunction::Sop(cubes) => {
            for (cube_index, cube) in cubes.iter().rev().enumerate() {
                if cube.literals.len() != node.fanins.len() {
                    return Err(IoCommandError::InvalidCover {
                        node: node_id,
                        cube: cube_index,
                    });
                }
                if cube_index != 0 {
                    line.push_str(" + ");
                }

                let mut first_literal = true;
                for (fanin_index, literal) in cube.literals.iter().enumerate() {
                    if *literal == IoLiteral::DontCare {
                        continue;
                    }
                    if !first_literal {
                        line.push_str(" * ");
                    }
                    if *literal == IoLiteral::Zero {
                        line.push('/');
                    }
                    line.push_str(network.node_name(node.fanins[fanin_index], short_name)?);
                    first_literal = false;
                }
                if first_literal {
                    line.push_str("VCC");
                }
            }
        }
    }

    Ok(Some(line))
}

pub fn print_nodes(
    network: &IoNetwork,
    negative: bool,
    names: &[String],
) -> Result<IoCommandOutput, IoCommandError> {
    let nodes = network.selected_nodes(names)?;
    let mut lines = Vec::new();
    for node in nodes {
        let prefix = if negative { "!" } else { "" };
        lines.push(format!(
            "{prefix}{}",
            format_node_equation(network, node, false)?
        ));
    }
    Ok(IoCommandOutput::with_stdout(lines))
}

pub fn print_io(network: &IoNetwork, names: &[String]) -> Result<IoCommandOutput, IoCommandError> {
    if !names.is_empty() {
        let mut lines = Vec::new();
        for node in network.selected_nodes(names)? {
            let data = network.node(node)?;
            lines.push(data.name.clone());
            lines.push(format!(
                "   inputs:  {}",
                names_for_nodes(network, &data.fanins)?.join(" ")
            ));
            let fanouts = data.fanouts.iter().copied().collect::<Vec<_>>();
            lines.push(format!(
                "   outputs: {}",
                names_for_nodes(network, &fanouts)?.join(" ")
            ));
        }
        return Ok(IoCommandOutput::with_stdout(lines));
    }

    let inputs = names_for_nodes(network, network.primary_inputs())?.join(" ");
    let outputs = names_for_nodes(network, network.primary_outputs())?.join(" ");
    Ok(IoCommandOutput::with_stdout(vec![
        format!("primary inputs:  {inputs}"),
        format!("primary outputs: {outputs}"),
    ]))
}

pub fn print_stats(
    network: &IoNetwork,
    factored_literals: bool,
    names: &[String],
) -> Result<IoCommandOutput, IoCommandError> {
    if !names.is_empty() {
        let mut lines = Vec::new();
        for node in network.selected_nodes(names)? {
            let data = network.node(node)?;
            match data.kind {
                IoNodeKind::PrimaryInput => {
                    lines.push(format!("{:<10} (primary input)", data.name))
                }
                IoNodeKind::PrimaryOutput => {
                    lines.push(format!("{:<10} (primary output)", data.name))
                }
                IoNodeKind::Internal => lines.push(format!(
                    "{:<10} {} terms, {} literals",
                    data.name,
                    cube_count(data),
                    literal_count(data)
                )),
            }
        }
        return Ok(IoCommandOutput::with_stdout(lines));
    }

    let sop_literals = network
        .nodes()
        .map(|(_, node)| literal_count(node))
        .sum::<usize>();
    let mut line = format!(
        "{:<14}\tpi={:2}\tpo={:2}\tnodes={:3}\tlatches={:2}\nlits(sop)={:4}",
        network.name(),
        network
            .primary_inputs()
            .iter()
            .filter(|id| network.node(**id).map(|n| n.is_real_pi).unwrap_or(false))
            .count(),
        network
            .primary_outputs()
            .iter()
            .filter(|id| network.node(**id).map(|n| n.is_real_po).unwrap_or(false))
            .count(),
        network
            .nodes()
            .filter(|(_, node)| node.kind == IoNodeKind::Internal)
            .count(),
        network.latches().len(),
        sop_literals
    );
    if factored_literals {
        line.push_str(&format!("\tlits(fac)={:4}", sop_literals));
    }

    Ok(IoCommandOutput::with_stdout(vec![line]))
}

pub fn print_alt_names(
    network: &IoNetwork,
    names: &[String],
) -> Result<IoCommandOutput, IoCommandError> {
    if names.is_empty() {
        return Err(IoCommandError::Usage {
            command: IoCommandKind::PrintAltName,
            usage: "usage: print_altname n1 n2 ...",
        });
    }

    let mut lines = Vec::new();
    for node in network.selected_nodes(names)? {
        let data = network.node(node)?;
        lines.push(format!("{} = {}", data.name, data.short_name));
    }

    Ok(IoCommandOutput::with_stdout(lines))
}

fn parse_read_command(kind: IoCommandKind, args: &[String]) -> Result<IoCommand, IoCommandError> {
    let mut append = false;
    let mut single_output = true;
    let mut operands = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-a" if operands.is_empty() => append = true,
            "-s" if operands.is_empty() => single_output = true,
            "-c" if operands.is_empty() => single_output = false,
            _ if arg.starts_with('-') && operands.is_empty() => {
                return Err(IoCommandError::Usage {
                    command: kind,
                    usage: read_usage(kind),
                });
            }
            _ => operands.push(arg.clone()),
        }
    }

    let filename = match operands.as_slice() {
        [] => "-".to_owned(),
        [file] => file.clone(),
        _ => {
            return Err(IoCommandError::Usage {
                command: kind,
                usage: read_usage(kind),
            });
        }
    };

    Ok(IoCommand::Read {
        kind,
        append,
        single_output,
        filename,
    })
}

fn parse_write_command(kind: IoCommandKind, args: &[String]) -> Result<IoCommand, IoCommandError> {
    let mut short_name = false;
    let mut net_list = false;
    let mut delays = false;
    let mut operands = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-s" if operands.is_empty() => short_name = true,
            "-n" if operands.is_empty() => net_list = true,
            "-d" if operands.is_empty() => delays = true,
            _ if arg.starts_with('-') && operands.is_empty() => {
                return Err(IoCommandError::Usage {
                    command: kind,
                    usage: write_usage(kind),
                });
            }
            _ => operands.push(arg.clone()),
        }
    }

    let filename = match operands.as_slice() {
        [] => "-".to_owned(),
        [file] => file.clone(),
        _ => {
            return Err(IoCommandError::Usage {
                command: kind,
                usage: write_usage(kind),
            });
        }
    };

    Ok(IoCommand::Write {
        kind,
        short_name,
        net_list,
        delays,
        filename,
    })
}

fn parse_write_pds_command(args: &[String]) -> Result<IoCommand, IoCommandError> {
    let mut short_name = false;
    let mut combinational = false;
    let mut debug = false;
    let mut operands = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-c" if operands.is_empty() => combinational = true,
            "-d" if operands.is_empty() => debug = true,
            "-s" if operands.is_empty() => short_name = true,
            _ if arg.starts_with('-') && operands.is_empty() => {
                return Err(IoCommandError::Usage {
                    command: IoCommandKind::WritePds,
                    usage: "usage: write_pds [-csd] [filename]",
                });
            }
            _ => operands.push(arg.clone()),
        }
    }

    let filename = match operands.as_slice() {
        [] => "-".to_owned(),
        [file] => file.clone(),
        _ => {
            return Err(IoCommandError::Usage {
                command: IoCommandKind::WritePds,
                usage: "usage: write_pds [-csd] [filename]",
            });
        }
    };

    Ok(IoCommand::WritePds {
        short_name,
        combinational,
        debug,
        filename,
    })
}

fn parse_print_command(args: &[String]) -> Result<IoCommand, IoCommandError> {
    let (negative, print_dc, nodes) = parse_flagged_nodes(
        args,
        "nd",
        IoCommandKind::Print,
        "usage: print [-d] [-n] n1 n2 ...",
    )?;
    Ok(IoCommand::Print {
        negative,
        print_dc,
        nodes,
    })
}

fn parse_print_io_command(args: &[String]) -> Result<IoCommand, IoCommandError> {
    let (_, print_dc, nodes) = parse_flagged_nodes(
        args,
        "d",
        IoCommandKind::PrintIo,
        "print_io [-d] [n1 n2 ...]",
    )?;
    Ok(IoCommand::PrintIo { print_dc, nodes })
}

fn parse_print_stats_command(args: &[String]) -> Result<IoCommand, IoCommandError> {
    let mut factored_literals = false;
    let mut print_dc = false;
    let mut nodes = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-f" if nodes.is_empty() => factored_literals = true,
            "-d" if nodes.is_empty() => print_dc = true,
            _ if arg.starts_with('-') && nodes.is_empty() => {
                return Err(IoCommandError::Usage {
                    command: IoCommandKind::PrintStats,
                    usage: "print_stats [-d] [-f] [n1 n2 ...]",
                });
            }
            _ => nodes.push(arg.clone()),
        }
    }

    Ok(IoCommand::PrintStats {
        factored_literals,
        print_dc,
        nodes,
    })
}

fn parse_reset_name_command(args: &[String]) -> Result<IoCommand, IoCommandError> {
    let mut short_names = true;
    let mut long_names = true;

    for arg in args {
        match arg.as_str() {
            "-s" => long_names = false,
            "-l" => short_names = false,
            _ => {
                return Err(IoCommandError::Usage {
                    command: IoCommandKind::ResetName,
                    usage: "usage: reset_names [-sl]",
                });
            }
        }
    }

    Ok(IoCommand::ResetName {
        short_names,
        long_names,
    })
}

fn parse_print_altname_command(args: &[String]) -> Result<IoCommand, IoCommandError> {
    if args.is_empty() {
        return Err(IoCommandError::Usage {
            command: IoCommandKind::PrintAltName,
            usage: "usage: print_altname n1 n2 ...",
        });
    }

    Ok(IoCommand::PrintAltName {
        nodes: args.to_vec(),
    })
}

fn parse_invert_io_command(args: &[String]) -> Result<IoCommand, IoCommandError> {
    if args.is_empty() {
        return Err(IoCommandError::Usage {
            command: IoCommandKind::InvertIo,
            usage: "usage: invert_io n1 n2 ...",
        });
    }

    Ok(IoCommand::InvertIo {
        nodes: args.to_vec(),
    })
}

fn parse_bdsyn_command(args: &[String]) -> Result<IoCommand, IoCommandError> {
    match args {
        [] => Ok(IoCommand::BdSyn {
            minimize_each_node: true,
        }),
        [value] if value == "1" => Ok(IoCommand::BdSyn {
            minimize_each_node: false,
        }),
        _ => Err(IoCommandError::Usage {
            command: IoCommandKind::BdSyn,
            usage: "usage: bdsyn [1]",
        }),
    }
}

fn parse_no_arg_command(kind: IoCommandKind, args: &[String]) -> Result<(), IoCommandError> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(IoCommandError::Usage {
            command: kind,
            usage: no_arg_usage(kind),
        })
    }
}

fn parse_flagged_nodes(
    args: &[String],
    allowed: &str,
    command: IoCommandKind,
    usage: &'static str,
) -> Result<(bool, bool, Vec<String>), IoCommandError> {
    let mut negative = false;
    let mut print_dc = false;
    let mut nodes = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-n" if allowed.contains('n') && nodes.is_empty() => negative = true,
            "-d" if allowed.contains('d') && nodes.is_empty() => print_dc = true,
            _ if arg.starts_with('-') && nodes.is_empty() => {
                return Err(IoCommandError::Usage { command, usage });
            }
            _ => nodes.push(arg.clone()),
        }
    }

    Ok((negative, print_dc, nodes))
}

fn selected_network(
    network: Option<&IoNetwork>,
    print_dc: bool,
) -> Result<&IoNetwork, IoCommandError> {
    let network = network.ok_or(IoCommandError::MissingNetwork)?;
    if print_dc {
        Ok(network.dc_network().unwrap_or(network))
    } else {
        Ok(network)
    }
}

fn read_usage(kind: IoCommandKind) -> &'static str {
    match kind {
        IoCommandKind::ReadPla => "usage: read_pla [-a] [-s] [file]",
        _ => "usage: read_command [-a] [file]",
    }
}

fn write_usage(kind: IoCommandKind) -> &'static str {
    match kind {
        IoCommandKind::WriteSlif => "usage: write_slif [-s] [-n] [-d] [filename]",
        IoCommandKind::WriteBlif => "usage: write_command [-s] [-n] [filename]",
        _ => "usage: write_command [-s] [filename]",
    }
}

fn no_arg_usage(kind: IoCommandKind) -> &'static str {
    match kind {
        IoCommandKind::ChangeName => "usage: chname",
        IoCommandKind::ForceInitZero => "usage: force_init_0",
        _ => "usage: command",
    }
}

fn push_pin(output: &mut String, name: &str, printed: &mut usize) {
    output.push_str(name);
    output.push(' ');
    *printed += 1;
    if *printed > 9 {
        output.push('\n');
        *printed = 0;
    }
}

fn io_node_should_be_printed(node: &IoNode) -> bool {
    node.kind != IoNodeKind::PrimaryInput
}

fn format_node_equation(
    network: &IoNetwork,
    node: IoNodeId,
    short_name: bool,
) -> Result<String, IoCommandError> {
    Ok(
        write_pds_sop(network, node, short_name)?.unwrap_or_else(|| {
            network
                .node(node)
                .map(|n| n.name.clone())
                .unwrap_or_default()
        }),
    )
}

fn names_for_nodes(network: &IoNetwork, nodes: &[IoNodeId]) -> Result<Vec<String>, IoCommandError> {
    nodes
        .iter()
        .map(|node| network.node(*node).map(|data| data.name.clone()))
        .collect()
}

fn cube_count(node: &IoNode) -> usize {
    match &node.function {
        IoFunction::Sop(cubes) => cubes.len(),
        IoFunction::Zero => 0,
        IoFunction::One | IoFunction::Buffer | IoFunction::Inverter => 1,
    }
}

fn literal_count(node: &IoNode) -> usize {
    match &node.function {
        IoFunction::Sop(cubes) => cubes
            .iter()
            .map(|cube| {
                cube.literals
                    .iter()
                    .filter(|literal| **literal != IoLiteral::DontCare)
                    .count()
            })
            .sum(),
        IoFunction::Inverter | IoFunction::Buffer => node.fanins.len(),
        IoFunction::Zero | IoFunction::One => 0,
    }
}

fn is_madeup_name(name: &str) -> bool {
    name.strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .is_some_and(|value| !value.is_empty() && value.chars().all(|item| item.is_ascii_digit()))
}

fn generated_short_name(index: usize) -> String {
    let letter = (b'a' + (index % 26) as u8) as char;
    let suffix = index / 26;
    if suffix == 0 {
        letter.to_string()
    } else {
        format!("{letter}{}", suffix - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingBackend {
        reads: Vec<String>,
        writes: Vec<String>,
    }

    impl IoCommandBackend for RecordingBackend {
        fn read_network(
            &mut self,
            kind: IoCommandKind,
            filename: &str,
            single_output: bool,
        ) -> Result<IoNetwork, IoCommandError> {
            self.reads
                .push(format!("{kind:?}:{filename}:{single_output}"));
            Ok(sample_network())
        }

        fn write_network(
            &mut self,
            kind: IoCommandKind,
            network: &IoNetwork,
            options: &WriteOptions,
        ) -> Result<Vec<String>, IoCommandError> {
            self.writes.push(format!(
                "{kind:?}:{}:{}:{}:{}",
                network.name(),
                options.filename,
                options.short_name,
                options.net_list
            ));
            Ok(vec!["written".to_owned()])
        }

        fn append_network(
            &mut self,
            current: &mut IoNetwork,
            new_network: IoNetwork,
        ) -> Result<(), IoCommandError> {
            current.set_name(format!("{}+{}", current.name(), new_network.name()));
            Ok(())
        }
    }

    fn sample_network() -> IoNetwork {
        let mut network = IoNetwork::new("demo");
        let a = network.add_primary_input("a").unwrap();
        let b = network.add_primary_input("b").unwrap();
        network.node_mut(a).unwrap().short_name = "i0".to_owned();
        network.node_mut(b).unwrap().short_name = "i1".to_owned();
        let n = network
            .add_internal(
                "n",
                [a, b],
                IoFunction::Sop(vec![
                    IoCube::new([IoLiteral::One, IoLiteral::Zero]),
                    IoCube::new([IoLiteral::DontCare, IoLiteral::One]),
                ]),
            )
            .unwrap();
        network.node_mut(n).unwrap().short_name = "x".to_owned();
        let y = network.add_primary_output("y", n).unwrap();
        network.node_mut(y).unwrap().short_name = "z".to_owned();
        network
    }

    #[test]
    fn registrations_match_legacy_init_io() {
        let names = io_command_registrations(false)
            .into_iter()
            .map(|command| (command.name, command.changes_network))
            .collect::<Vec<_>>();

        assert!(names.contains(&("read_blif", true)));
        assert!(names.contains(&("write_blif", false)));
        assert!(names.contains(&("write_pds", false)));
        assert!(names.contains(&("invert_io", true)));
        assert!(!names.iter().any(|(name, _)| *name == "plot_blif"));
        assert!(
            io_command_registrations(true)
                .iter()
                .any(|command| command.name == "plot_blif")
        );
    }

    #[test]
    fn parses_read_pla_options_and_default_filename() {
        assert_eq!(
            parse_io_command("read_pla", ["-a", "-c"]).unwrap(),
            IoCommand::Read {
                kind: IoCommandKind::ReadPla,
                append: true,
                single_output: false,
                filename: "-".to_owned(),
            }
        );
    }

    #[test]
    fn parses_write_slif_options() {
        assert_eq!(
            parse_io_command("write_slif", ["-s", "-n", "-d", "out.slif"]).unwrap(),
            IoCommand::Write {
                kind: IoCommandKind::WriteSlif,
                short_name: true,
                net_list: true,
                delays: true,
                filename: "out.slif".to_owned(),
            }
        );
    }

    #[test]
    fn parses_print_and_reset_commands() {
        assert_eq!(
            parse_io_command("print", ["-d", "-n", "a"]).unwrap(),
            IoCommand::Print {
                negative: true,
                print_dc: true,
                nodes: vec!["a".to_owned()],
            }
        );
        assert_eq!(
            parse_io_command("reset_name", ["-s"]).unwrap(),
            IoCommand::ResetName {
                short_names: true,
                long_names: false,
            }
        );
    }

    #[test]
    fn read_dispatch_replaces_or_appends_network() {
        let mut backend = RecordingBackend::default();
        let mut network = None;
        let mut name_mode = NameMode::Long;
        let command = parse_io_command("read_blif", ["input.blif"]).unwrap();

        dispatch_io_command(&mut network, &mut name_mode, &mut backend, &command).unwrap();

        assert_eq!(backend.reads, vec!["ReadBlif:input.blif:true"]);
        assert_eq!(network.as_ref().unwrap().name(), "demo");

        let command = parse_io_command("read_eqn", ["-a", "other.eqn"]).unwrap();
        dispatch_io_command(&mut network, &mut name_mode, &mut backend, &command).unwrap();
        assert_eq!(network.as_ref().unwrap().name(), "demo+demo");
    }

    #[test]
    fn write_dispatch_uses_backend_with_options() {
        let mut backend = RecordingBackend::default();
        let mut network = Some(sample_network());
        let mut name_mode = NameMode::Long;
        let command = parse_io_command("write_blif", ["-s", "-n", "out.blif"]).unwrap();

        let output =
            dispatch_io_command(&mut network, &mut name_mode, &mut backend, &command).unwrap();

        assert_eq!(output.stdout, vec!["written"]);
        assert_eq!(backend.writes, vec!["WriteBlif:demo:out.blif:true:true"]);
    }

    #[test]
    fn palasm_writer_preserves_pinlist_equations_and_latches() {
        let mut network = sample_network();
        let latch_input = network.find_node("y").unwrap();
        let latch_output = network.find_node("a").unwrap();
        network
            .add_latch(IoLatch::new(
                latch_input,
                latch_output,
                LatchInitialValue::One,
            ))
            .unwrap();

        let output = write_palasm(&network, false, false).unwrap();

        assert!(output.contains("CHIP dummy LCA"));
        assert!(output.contains(";PINLIST"));
        assert!(output.contains("a b y"));
        assert!(output.contains("clock"));
        assert!(output.contains("n = b + a * /b"));
        assert!(output.contains("a := y"));
        assert!(output.contains("a.CLKF = clock"));
    }

    #[test]
    fn print_io_and_stats_match_legacy_shape() {
        let network = sample_network();

        assert_eq!(
            print_io(&network, &[]).unwrap().stdout,
            vec!["primary inputs:  a b", "primary outputs: y"]
        );
        assert_eq!(
            print_stats(&network, false, &["n".to_owned()])
                .unwrap()
                .stdout,
            vec!["n          2 terms, 3 literals"]
        );
    }

    #[test]
    fn change_name_toggles_name_mode_message() {
        let mut backend = RecordingBackend::default();
        let mut network = Some(sample_network());
        let mut mode = NameMode::Long;
        let output = dispatch_io_command(
            &mut network,
            &mut mode,
            &mut backend,
            &IoCommand::ChangeName,
        )
        .unwrap();

        assert_eq!(mode, NameMode::Short);
        assert_eq!(output.stdout, vec!["changing to short-name mode"]);
    }

    #[test]
    fn invert_io_inserts_inverter_and_drops_dc_network() {
        let mut network = sample_network();
        network.set_dc_network(Some(IoNetwork::new("dc")));
        let mut backend = RecordingBackend::default();
        let mut network = Some(network);
        let mut mode = NameMode::Long;
        let command = parse_io_command("invert_io", ["y"]).unwrap();

        dispatch_io_command(&mut network, &mut mode, &mut backend, &command).unwrap();

        let network = network.unwrap();
        assert!(network.dc_network().is_none());
        assert!(
            network
                .nodes()
                .any(|(_, node)| node.function == IoFunction::Inverter)
        );
        let y = network.find_node("y").unwrap();
        let driver = network.node(y).unwrap().fanins[0];
        assert_eq!(network.node(driver).unwrap().function, IoFunction::Inverter);
    }

    #[test]
    fn force_init_zero_inverts_one_initialized_latches() {
        let mut network = sample_network();
        let y = network.find_node("y").unwrap();
        let a = network.find_node("a").unwrap();
        network
            .add_latch(IoLatch::new(y, a, LatchInitialValue::One))
            .unwrap();

        network.force_initial_state_to_zero().unwrap();

        assert_eq!(network.latches()[0].initial_value, LatchInitialValue::Zero);
        assert_eq!(
            network
                .nodes()
                .filter(|(_, node)| node.function == IoFunction::Inverter)
                .count(),
            2
        );
    }

    #[test]
    fn source_contains_no_dependency_metadata_or_c_abi_export() {
        let source = include_str!("com_io.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
