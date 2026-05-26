//! Native Rust read utilities for SIS IO readers.
//!
//! The original `read_util.c` combines parser state, node lookup/creation,
//! reader cleanup, delay attributes, model dependency patching, and external
//! don't-care validation. This port keeps those responsibilities as safe owned
//! Rust types so higher-level BLIF/SLIF readers can share the behavior without
//! relying on global C state or ABI entry points.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadContext {
    line: usize,
    filename: Option<String>,
    diagnostics: Vec<String>,
}

impl Default for ReadContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadContext {
    pub fn new() -> Self {
        Self {
            line: 1,
            filename: None,
            diagnostics: Vec::new(),
        }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    pub fn increment_line(&mut self) {
        self.line += 1;
    }

    pub fn register_filename(&mut self, filename: Option<impl Into<String>>) {
        self.line = 1;
        self.filename = filename.map(Into::into);
    }

    pub fn read_error(&mut self, message: impl AsRef<str>) {
        let message = message.as_ref();
        if let Some(filename) = &self.filename {
            self.diagnostics
                .push(format!("\"{filename}\", line {}: {message}", self.line));
        } else {
            self.diagnostics.push(message.to_owned());
        }
    }

    pub fn clear_diagnostics(&mut self) {
        self.diagnostics.clear();
    }
}

pub fn filename_to_netname(filename: impl AsRef<Path>) -> String {
    filename
        .as_ref()
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenCursor {
    remainder: Option<String>,
}

impl TokenCursor {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            remainder: Some(input.into()),
        }
    }

    pub fn reset(&mut self, input: impl Into<String>) {
        self.remainder = Some(input.into());
    }

    pub fn next_token(&mut self) -> Option<String> {
        let remainder = self.remainder.take()?;
        if remainder.is_empty() {
            return None;
        }

        let mut split = remainder.splitn(2, ' ');
        let token = split.next().unwrap_or_default().to_owned();
        self.remainder = split.next().map(str::to_owned);
        if token.is_empty() {
            self.next_token()
        } else {
            Some(token)
        }
    }

    pub fn rest(&self) -> Option<&str> {
        self.remainder.as_deref()
    }
}

pub fn gettoken(input: Option<&str>, cursor: &mut Option<TokenCursor>) -> Option<String> {
    if let Some(input) = input {
        *cursor = Some(TokenCursor::new(input));
    }

    cursor.as_mut()?.next_token()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ReadNodeId(usize);

impl ReadNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadNodeFunction {
    Undefined,
    Constant(bool),
    Buffer,
    Literal { source: ReadNodeId, phase: bool },
    Sop(Vec<Vec<Option<bool>>>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DelayPhase {
    Inverting,
    NonInverting,
    Neither,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DelayParam {
    ArrivalRise,
    ArrivalFall,
    RequiredRise,
    RequiredFall,
    DriveRise,
    DriveFall,
    OutputLoad,
    MaxInputLoad,
    DefaultArrivalRise,
    DefaultArrivalFall,
    DefaultRequiredRise,
    DefaultRequiredFall,
    DefaultDriveRise,
    DefaultDriveFall,
    DefaultOutputLoad,
    DefaultMaxInputLoad,
    BlockRise,
    BlockFall,
    InputLoad,
    Phase,
    WireLoadSlope,
    AddWireLoad,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReadNode {
    pub name: String,
    pub kind: ReadNodeKind,
    pub function: ReadNodeFunction,
    pub fanins: Vec<ReadNodeId>,
    pub fanouts: BTreeSet<ReadNodeId>,
    pub delays: BTreeMap<DelayParam, f64>,
    pub is_real_pi: bool,
    pub is_real_po: bool,
    pub latch_output: bool,
}

impl ReadNode {
    pub fn new(name: impl Into<String>, kind: ReadNodeKind) -> Self {
        let is_real_pi = kind == ReadNodeKind::PrimaryInput;
        let is_real_po = kind == ReadNodeKind::PrimaryOutput;
        Self {
            name: name.into(),
            kind,
            function: ReadNodeFunction::Undefined,
            fanins: Vec::new(),
            fanouts: BTreeSet::new(),
            delays: BTreeMap::new(),
            is_real_pi,
            is_real_po,
            latch_output: false,
        }
    }

    pub fn constant(name: impl Into<String>, value: bool) -> Self {
        let mut node = Self::new(name, ReadNodeKind::Internal);
        node.function = ReadNodeFunction::Constant(value);
        node
    }

    pub fn buffer(name: impl Into<String>, fanin: ReadNodeId) -> Self {
        let mut node = Self::new(name, ReadNodeKind::Internal);
        node.function = ReadNodeFunction::Buffer;
        node.fanins.push(fanin);
        node
    }

    pub fn is_undriven_internal(&self) -> bool {
        self.kind == ReadNodeKind::Internal && self.function == ReadNodeFunction::Undefined
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReadNetwork {
    name: String,
    nodes: Vec<Option<ReadNode>>,
    order: Vec<ReadNodeId>,
    inputs: Vec<ReadNodeId>,
    outputs: Vec<ReadNodeId>,
    name_table: BTreeMap<String, ReadNodeId>,
    default_delays: BTreeMap<DelayParam, f64>,
    area: Option<f64>,
    dc_network: Option<Box<ReadNetwork>>,
    latch_order: Vec<ReadNodeId>,
    stg_names_present: bool,
}

impl Default for ReadNetwork {
    fn default() -> Self {
        Self::new("unknown")
    }
}

impl ReadNetwork {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            order: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            name_table: BTreeMap::new(),
            default_delays: BTreeMap::new(),
            area: None,
            dc_network: None,
            latch_order: Vec::new(),
            stg_names_present: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn area(&self) -> Option<f64> {
        self.area
    }

    pub fn default_delay(&self, param: DelayParam) -> Option<f64> {
        self.default_delays.get(&param).copied()
    }

    pub fn dc_network(&self) -> Option<&ReadNetwork> {
        self.dc_network.as_deref()
    }

    pub fn set_dc_network(&mut self, dc_network: Option<ReadNetwork>) {
        self.dc_network = dc_network.map(Box::new);
    }

    pub fn node(&self, id: ReadNodeId) -> ReadResult<&ReadNode> {
        self.nodes
            .get(id.index())
            .and_then(Option::as_ref)
            .ok_or(ReadError::MissingNode(id))
    }

    pub fn node_mut(&mut self, id: ReadNodeId) -> ReadResult<&mut ReadNode> {
        self.nodes
            .get_mut(id.index())
            .and_then(Option::as_mut)
            .ok_or(ReadError::MissingNode(id))
    }

    pub fn nodes(&self) -> impl Iterator<Item = (ReadNodeId, &ReadNode)> {
        self.order.iter().filter_map(|id| {
            self.nodes
                .get(id.index())
                .and_then(Option::as_ref)
                .map(|node| (*id, node))
        })
    }

    pub fn primary_inputs(&self) -> &[ReadNodeId] {
        &self.inputs
    }

    pub fn primary_outputs(&self) -> &[ReadNodeId] {
        &self.outputs
    }

    pub fn find_node(&self, name: &str) -> Option<ReadNodeId> {
        self.name_table.get(name).copied()
    }

    pub fn add_node(&mut self, node: ReadNode) -> ReadResult<ReadNodeId> {
        if self.name_table.contains_key(&node.name) {
            return Err(ReadError::DuplicateName(node.name));
        }

        for fanin in &node.fanins {
            self.node(*fanin)?;
        }

        let id = ReadNodeId(self.nodes.len());
        if node.kind == ReadNodeKind::PrimaryInput {
            self.inputs.push(id);
        } else if node.kind == ReadNodeKind::PrimaryOutput {
            self.outputs.push(id);
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

    pub fn change_node_name(
        &mut self,
        node: ReadNodeId,
        name: impl Into<String>,
    ) -> ReadResult<()> {
        let name = name.into();
        if let Some(existing) = self.name_table.get(&name) {
            if *existing != node {
                return Err(ReadError::DuplicateName(name));
            }
        }

        let old_name = self.node(node)?.name.clone();
        self.name_table.remove(&old_name);
        self.node_mut(node)?.name = name.clone();
        self.name_table.insert(name, node);
        Ok(())
    }

    pub fn change_node_kind(&mut self, node: ReadNodeId, kind: ReadNodeKind) -> ReadResult<()> {
        let old_kind = self.node(node)?.kind;
        if old_kind == kind {
            return Ok(());
        }

        self.inputs.retain(|item| *item != node);
        self.outputs.retain(|item| *item != node);

        if kind == ReadNodeKind::PrimaryInput {
            self.inputs.push(node);
        } else if kind == ReadNodeKind::PrimaryOutput {
            self.outputs.push(node);
        }

        let target = self.node_mut(node)?;
        target.kind = kind;
        target.is_real_pi = kind == ReadNodeKind::PrimaryInput;
        target.is_real_po = kind == ReadNodeKind::PrimaryOutput;
        Ok(())
    }

    pub fn set_node_function(
        &mut self,
        node: ReadNodeId,
        function: ReadNodeFunction,
    ) -> ReadResult<()> {
        self.node_mut(node)?.function = function;
        Ok(())
    }

    pub fn patch_fanin(
        &mut self,
        node: ReadNodeId,
        old_fanin: ReadNodeId,
        new_fanin: ReadNodeId,
    ) -> ReadResult<()> {
        self.node(old_fanin)?;
        self.node(new_fanin)?;

        let replaced = {
            let target = self.node_mut(node)?;
            let mut replaced = false;
            for fanin in &mut target.fanins {
                if *fanin == old_fanin {
                    *fanin = new_fanin;
                    replaced = true;
                }
            }
            replaced
        };

        if replaced {
            self.node_mut(old_fanin)?.fanouts.remove(&node);
            self.node_mut(new_fanin)?.fanouts.insert(node);
        }

        Ok(())
    }

    pub fn delete_node(&mut self, node: ReadNodeId) -> ReadResult<ReadNode> {
        let removed = self.nodes[node.index()]
            .take()
            .ok_or(ReadError::MissingNode(node))?;

        self.order.retain(|item| *item != node);
        self.inputs.retain(|item| *item != node);
        self.outputs.retain(|item| *item != node);
        self.name_table.remove(&removed.name);

        for fanin in &removed.fanins {
            if let Some(fanin_node) = self.nodes.get_mut(fanin.index()).and_then(Option::as_mut) {
                fanin_node.fanouts.remove(&node);
            }
        }

        for fanout in &removed.fanouts {
            if let Some(fanout_node) = self.nodes.get_mut(fanout.index()).and_then(Option::as_mut) {
                fanout_node.fanins.retain(|fanin| *fanin != node);
            }
        }

        Ok(removed)
    }

    pub fn add_primary_output(&mut self, driver: ReadNodeId) -> ReadResult<ReadNodeId> {
        self.node(driver)?;
        let name = self.node(driver)?.name.clone();
        let mut output = ReadNode::new(name, ReadNodeKind::PrimaryOutput);
        output.fanins.push(driver);
        self.add_node_with_unique_name(output)
    }

    pub fn add_primary_output_named(
        &mut self,
        driver: ReadNodeId,
        name: impl Into<String>,
    ) -> ReadResult<ReadNodeId> {
        self.node(driver)?;
        let mut output = ReadNode::new(name, ReadNodeKind::PrimaryOutput);
        output.fanins.push(driver);
        self.add_node(output)
    }

    fn add_node_with_unique_name(&mut self, mut node: ReadNode) -> ReadResult<ReadNodeId> {
        if self.name_table.contains_key(&node.name) {
            let base = node.name.clone();
            let mut index = 0usize;
            loop {
                let candidate = format!("{base}_{index}");
                if !self.name_table.contains_key(&candidate) {
                    node.name = candidate;
                    break;
                }
                index += 1;
            }
        }

        self.add_node(node)
    }

    pub fn set_area(&mut self, context: &mut ReadContext, area: f64) -> ReadResult<()> {
        if self.area.is_some() {
            context.read_error("area given twice for same model");
            return Err(ReadError::AreaAlreadyGiven);
        }

        self.area = Some(area);
        Ok(())
    }

    pub fn set_default_delay(&mut self, param: DelayParam, value: f64) {
        self.default_delays.insert(param, value);
    }

    pub fn set_stg_names_present(&mut self, value: bool) {
        self.stg_names_present = value;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadError {
    DuplicateName(String),
    MissingNode(ReadNodeId),
    MissingName(String),
    MissingToken(String),
    InvalidNumber(String),
    InvalidDelayKeyword(String),
    InvalidDelayPhase(String),
    WrongArgumentCount(String),
    NotInputNode(String),
    NotOutputNode(String),
    AreaAlreadyGiven,
    NetworkCycle(Vec<String>),
    MissingDcNetwork,
    DcOutputOnly(String),
    DcInputOnly(String),
    InvalidLatchOrder(String),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateName(name) => write!(f, "duplicate node name {name}"),
            Self::MissingNode(node) => write!(f, "missing node {}", node.index()),
            Self::MissingName(name) => write!(f, "missing node named {name}"),
            Self::MissingToken(context) => write!(f, "missing token in {context}"),
            Self::InvalidNumber(value) => write!(f, "invalid number {value}"),
            Self::InvalidDelayKeyword(keyword) => write!(f, "invalid delay keyword {keyword}"),
            Self::InvalidDelayPhase(phase) => write!(f, "invalid delay phase {phase}"),
            Self::WrongArgumentCount(keyword) => write!(f, "wrong number arguments to {keyword}"),
            Self::NotInputNode(name) => write!(f, "node {name} is not a primary input"),
            Self::NotOutputNode(name) => write!(f, "node {name} is not a primary output"),
            Self::AreaAlreadyGiven => write!(f, "area given twice for same model"),
            Self::NetworkCycle(names) => write!(f, "cyclic model dependency involving {names:?}"),
            Self::MissingDcNetwork => write!(f, "missing external don't-care network"),
            Self::DcOutputOnly(name) => {
                write!(
                    f,
                    "output {name} appears only in the external don't-care network"
                )
            }
            Self::DcInputOnly(name) => {
                write!(
                    f,
                    "input {name} appears only in the external don't-care network"
                )
            }
            Self::InvalidLatchOrder(name) => write!(f, "invalid latch order entry {name}"),
        }
    }
}

impl Error for ReadError {}

pub type ReadResult<T> = Result<T, ReadError>;

pub fn read_find_or_create_node(network: &mut ReadNetwork, name: &str) -> ReadResult<ReadNodeId> {
    let node = match network.find_node(name) {
        Some(node) => node,
        None => network.add_node(ReadNode::new(name, ReadNodeKind::Internal))?,
    };

    if network.node(node)?.kind == ReadNodeKind::PrimaryOutput {
        return network
            .node(node)?
            .fanins
            .first()
            .copied()
            .ok_or(ReadError::MissingNode(node));
    }

    Ok(node)
}

pub fn read_slif_find_or_create_node(
    network: &mut ReadNetwork,
    name: &str,
    output_named_complement: bool,
) -> ReadResult<ReadNodeId> {
    if let Some(node) = network.find_node(name) {
        return dereference_primary_output(network, node);
    }

    let complement = name.len() > 1 && name.ends_with('\'');
    let node = if complement {
        let positive_name = &name[..name.len() - 1];
        let positive = read_find_or_create_node(network, positive_name)?;
        if output_named_complement {
            let created = network.add_node(ReadNode::new(name, ReadNodeKind::Internal))?;
            network.set_node_function(
                positive,
                ReadNodeFunction::Literal {
                    source: created,
                    phase: false,
                },
            )?;
            created
        } else {
            let mut created = ReadNode::new(name, ReadNodeKind::Internal);
            created.function = ReadNodeFunction::Literal {
                source: positive,
                phase: false,
            };
            created.fanins.push(positive);
            network.add_node(created)?
        }
    } else {
        network.add_node(ReadNode::new(name, ReadNodeKind::Internal))?
    };

    if name == "0" {
        network.set_node_function(node, ReadNodeFunction::Constant(false))?;
    } else if name == "1" {
        network.set_node_function(node, ReadNodeFunction::Constant(true))?;
    }

    dereference_primary_output(network, node)
}

pub fn read_change_madeup_name(network: &mut ReadNetwork, node: ReadNodeId) -> ReadResult<bool> {
    let Some(index) = madeup_name_index(&network.node(node)?.name) else {
        return Ok(false);
    };

    network.change_node_name(node, format!(" {index}"))?;
    Ok(true)
}

pub fn read_check_io_list(
    context: &mut ReadContext,
    network: &mut ReadNetwork,
    po_list: &mut Vec<ReadNodeId>,
    print_warning: bool,
) -> ReadResult<()> {
    let (relax_i, relax_o) = compute_relax(network, po_list);
    let nodes = network.order.clone();
    for node_id in nodes {
        let node = network.node(node_id)?.clone();
        if node.is_undriven_internal() {
            if relax_i {
                network.change_node_kind(node_id, ReadNodeKind::PrimaryInput)?;
            } else {
                if print_warning {
                    context.read_error(format!(
                        "Warning: network `{}`, node \"{}\" is not driven (zero assumed)",
                        network.name(),
                        node.name
                    ));
                }
                network.set_node_function(node_id, ReadNodeFunction::Constant(false))?;
            }
        } else if node.kind != ReadNodeKind::PrimaryOutput
            && node.fanouts.is_empty()
            && !po_list.contains(&node_id)
        {
            if relax_o {
                po_list.push(node_id);
            } else if print_warning {
                context.read_error(format!(
                    "Warning: network `{}`, node \"{}\" does not fanout",
                    network.name(),
                    node.name
                ));
            }
        }
    }

    Ok(())
}

pub fn read_hack_outputs(
    context: &mut ReadContext,
    network: &mut ReadNetwork,
    po_list: &[ReadNodeId],
) -> ReadResult<Vec<ReadNodeId>> {
    let mut outputs = Vec::new();
    for node in po_list {
        let output = network.add_primary_output(*node)?;
        outputs.push(output);
        if network.node(*node)?.kind == ReadNodeKind::PrimaryInput
            && !network.node(*node)?.latch_output
        {
            let po_name = network.node(output)?.name.clone();
            let input_name = format!("IN-{po_name}");
            context.read_error(format!(
                "Warning: input and output named \"{po_name}\":  renaming input \"{input_name}\""
            ));
            network.change_node_name(*node, input_name)?;
        }
    }

    Ok(outputs)
}

pub fn read_cleanup_buffers(network: &mut ReadNetwork) -> ReadResult<Vec<ReadNodeId>> {
    let outputs = network.outputs.clone();
    let mut removed = Vec::new();
    for output in outputs {
        let Some(buffer) = network.node(output)?.fanins.first().copied() else {
            continue;
        };
        if network.node(buffer)?.function != ReadNodeFunction::Buffer {
            continue;
        }
        let Some(fanin) = network.node(buffer)?.fanins.first().copied() else {
            continue;
        };
        if network.node(fanin)?.latch_output {
            continue;
        }

        let fanouts = network
            .node(buffer)?
            .fanouts
            .iter()
            .copied()
            .collect::<Vec<_>>();
        for fanout in fanouts {
            network.patch_fanin(fanout, buffer, fanin)?;
        }
        network.delete_node(buffer)?;
        removed.push(buffer);
    }

    Ok(removed)
}

pub fn read_check_control_signals(
    context: &mut ReadContext,
    network: &ReadNetwork,
    controls: &mut [Option<ReadNodeId>],
) {
    let mut seen = BTreeSet::new();
    for control in controls {
        let Some(control_id) = *control else {
            continue;
        };
        if !seen.insert(control_id) {
            *control = None;
            continue;
        }
        let constant = network
            .node(control_id)
            .ok()
            .is_some_and(|node| matches!(node.function, ReadNodeFunction::Constant(_)));
        if constant {
            context.read_error(format!(
                "Warning: network `{}`, latch control `{}` is constant (disconnecting)",
                network.name(),
                network
                    .node(control_id)
                    .map(|node| node.name.as_str())
                    .unwrap_or("<missing>")
            ));
            *control = None;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayReadStatus {
    NotDelayRelated,
    DelayRelated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlifAttributeMode {
    NotSlif,
    Local,
    Global,
}

pub fn read_delay(
    context: &mut ReadContext,
    network: &mut ReadNetwork,
    po_list: &[ReadNodeId],
    word: &str,
    rest: &str,
    slif_mode: SlifAttributeMode,
) -> ReadResult<DelayReadStatus> {
    match word {
        "area" | "cost" => {
            if slif_mode == SlifAttributeMode::Local {
                context.read_error("area is a global attribute");
                return Err(ReadError::InvalidDelayKeyword(word.to_owned()));
            }
            let area = parse_one_number(rest)?;
            network.set_area(context, area)?;
            Ok(DelayReadStatus::DelayRelated)
        }
        "delay" => {
            let mut cursor = TokenCursor::new(rest);
            let target = if slif_mode == SlifAttributeMode::Global {
                None
            } else {
                Some(read_input_node(context, &mut cursor, network, word)?)
            };
            let phase_word = require_token(&mut cursor, word)?;
            let load = parse_token_number(&mut cursor, word)?;
            let max_load = parse_token_number(&mut cursor, word)?;
            let block_rise = parse_token_number(&mut cursor, word)?;
            let drive_rise = parse_token_number(&mut cursor, word)?;
            let block_fall = parse_token_number(&mut cursor, word)?;
            let drive_fall = parse_token_number(&mut cursor, word)?;
            let phase = match phase_word.as_str() {
                "INV" => DelayPhase::Inverting,
                "NONINV" => DelayPhase::NonInverting,
                "UNKNOWN" => DelayPhase::Neither,
                value => {
                    context.read_error("bad phase specification in .delay");
                    return Err(ReadError::InvalidDelayPhase(value.to_owned()));
                }
            };

            if let Some(node) = target {
                read_delay_set_parameters(
                    network, node, block_rise, block_fall, drive_rise, drive_fall, load, max_load,
                    phase,
                )?;
            } else {
                for node in network.primary_inputs().to_vec() {
                    read_delay_set_parameters(
                        network, node, block_rise, block_fall, drive_rise, drive_fall, load,
                        max_load, phase,
                    )?;
                }
            }
            Ok(DelayReadStatus::DelayRelated)
        }
        "wire_load_slope" => {
            if slif_mode == SlifAttributeMode::Local {
                context.read_error("wire_load_slope is a global attribute");
            }
            network.set_default_delay(DelayParam::WireLoadSlope, parse_one_number(rest)?);
            Ok(DelayReadStatus::DelayRelated)
        }
        "wire" => {
            network.default_delays.remove(&DelayParam::AddWireLoad);
            for token in rest.split_whitespace() {
                network.set_default_delay(DelayParam::AddWireLoad, parse_number(token)?);
            }
            Ok(DelayReadStatus::DelayRelated)
        }
        _ => read_standard_delay(context, network, po_list, word, rest, slif_mode),
    }
}

fn read_standard_delay(
    context: &mut ReadContext,
    network: &mut ReadNetwork,
    po_list: &[ReadNodeId],
    word: &str,
    rest: &str,
    slif_mode: SlifAttributeMode,
) -> ReadResult<DelayReadStatus> {
    let (default, keyword) = match word.strip_prefix("default_") {
        Some(keyword) => (true, keyword),
        None => (slif_mode == SlifAttributeMode::Global, word),
    };

    let mut cursor = TokenCursor::new(rest);
    let target = if default {
        None
    } else if keyword == "input_arrival" || keyword == "input_drive" || keyword == "max_input_load"
    {
        Some(read_input_node(context, &mut cursor, network, keyword)?)
    } else if keyword == "output_required" || keyword == "output_load" {
        Some(read_output_node(
            context,
            &mut cursor,
            network,
            po_list,
            keyword,
        )?)
    } else {
        return Ok(DelayReadStatus::NotDelayRelated);
    };

    let params = match keyword {
        "input_arrival" => {
            if default {
                (
                    DelayParam::DefaultArrivalRise,
                    Some(DelayParam::DefaultArrivalFall),
                )
            } else {
                (DelayParam::ArrivalRise, Some(DelayParam::ArrivalFall))
            }
        }
        "output_required" => {
            if default {
                (
                    DelayParam::DefaultRequiredRise,
                    Some(DelayParam::DefaultRequiredFall),
                )
            } else {
                (DelayParam::RequiredRise, Some(DelayParam::RequiredFall))
            }
        }
        "input_drive" => {
            if default {
                (
                    DelayParam::DefaultDriveRise,
                    Some(DelayParam::DefaultDriveFall),
                )
            } else {
                (DelayParam::DriveRise, Some(DelayParam::DriveFall))
            }
        }
        "output_load" => {
            if default {
                (DelayParam::DefaultOutputLoad, None)
            } else {
                (DelayParam::OutputLoad, None)
            }
        }
        "max_input_load" => {
            if default {
                (DelayParam::DefaultMaxInputLoad, None)
            } else {
                (DelayParam::MaxInputLoad, None)
            }
        }
        _ => unreachable!(),
    };

    let first = parse_token_number(&mut cursor, keyword)?;
    let second = match params.1 {
        Some(_) => Some(parse_token_number(&mut cursor, keyword)?),
        None => None,
    };
    read_delay_common(network, target, params.0, params.1, first, second);
    Ok(DelayReadStatus::DelayRelated)
}

fn read_delay_set_parameters(
    network: &mut ReadNetwork,
    node: ReadNodeId,
    block_rise: f64,
    block_fall: f64,
    drive_rise: f64,
    drive_fall: f64,
    load: f64,
    max_load: f64,
    phase: DelayPhase,
) -> ReadResult<()> {
    let phase = match phase {
        DelayPhase::Inverting => -1.0,
        DelayPhase::NonInverting => 1.0,
        DelayPhase::Neither => 0.0,
    };
    let node = network.node_mut(node)?;
    node.delays.insert(DelayParam::BlockRise, block_rise);
    node.delays.insert(DelayParam::BlockFall, block_fall);
    node.delays.insert(DelayParam::DriveRise, drive_rise);
    node.delays.insert(DelayParam::DriveFall, drive_fall);
    node.delays.insert(DelayParam::InputLoad, load);
    node.delays.insert(DelayParam::MaxInputLoad, max_load);
    node.delays.insert(DelayParam::Phase, phase);
    Ok(())
}

fn read_delay_common(
    network: &mut ReadNetwork,
    node: Option<ReadNodeId>,
    first_param: DelayParam,
    second_param: Option<DelayParam>,
    first: f64,
    second: Option<f64>,
) {
    if let Some(node) = node {
        if let Ok(target) = network.node_mut(node) {
            target.delays.insert(first_param, first);
            if let (Some(second_param), Some(second)) = (second_param, second) {
                target.delays.insert(second_param, second);
            }
        }
    } else {
        network.set_default_delay(first_param, first);
        if let (Some(second_param), Some(second)) = (second_param, second) {
            network.set_default_delay(second_param, second);
        }
    }
}

pub fn read_delay_cleanup(network: &mut ReadNetwork) -> ReadResult<()> {
    for output in network.outputs.clone() {
        if !network.node(output)?.is_real_po {
            continue;
        }
        let Some(fanin) = network.node(output)?.fanins.first().copied() else {
            continue;
        };
        if network.node(fanin)?.latch_output {
            let target = network.node_mut(fanin)?;
            target.delays.remove(&DelayParam::ArrivalRise);
            target.delays.remove(&DelayParam::ArrivalFall);
        }
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatchInfo {
    pub netname: String,
    pub actuals: Vec<String>,
    pub formals: Option<Vec<String>>,
    pub inputs: Option<usize>,
}

impl PatchInfo {
    pub fn new(netname: impl Into<String>) -> Self {
        Self {
            netname: netname.into(),
            actuals: Vec::new(),
            formals: None,
            inputs: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelInfo {
    pub network: Option<ReadNetwork>,
    pub po_list: Vec<ReadNodeId>,
    pub latch_order_list: Vec<ReadNodeId>,
    pub patch_lists: Vec<PatchInfo>,
    pub depends_on: usize,
    pub library: bool,
}

impl Default for ModelInfo {
    fn default() -> Self {
        Self {
            network: None,
            po_list: Vec::new(),
            latch_order_list: Vec::new(),
            patch_lists: Vec::new(),
            depends_on: 0,
            library: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelTable {
    models: BTreeMap<String, ModelInfo>,
}

impl ModelTable {
    pub fn find_or_create_model(&mut self, name: impl Into<String>) -> &mut ModelInfo {
        self.models.entry(name.into()).or_default()
    }

    pub fn get(&self, name: &str) -> Option<&ModelInfo> {
        self.models.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut ModelInfo> {
        self.models.get_mut(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.models.keys().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }
}

pub fn read_find_or_create_model<'a>(models: &'a mut ModelTable, name: &str) -> &'a mut ModelInfo {
    models.find_or_create_model(name)
}

pub fn read_special_case_for_stg(network: &ReadNetwork) -> bool {
    network
        .nodes()
        .filter(|(_, node)| node.kind == ReadNodeKind::PrimaryInput)
        .all(|(_, node)| node.fanouts.is_empty())
}

pub fn read_set_latch_order(
    context: &mut ReadContext,
    network: &mut ReadNetwork,
    latch_order_list: Vec<ReadNodeId>,
) -> ReadResult<()> {
    let has_order = !latch_order_list.is_empty();
    if !network.stg_names_present {
        if has_order {
            context.read_error(format!(
                "model `{}`: .latch_order unnecessary",
                network.name()
            ));
        }
        return Ok(());
    }

    if !has_order && !read_special_case_for_stg(network) {
        context.read_error(format!(
            "model `{}`: .latch_order & .code must be given w/ stg",
            network.name()
        ));
        return Err(ReadError::InvalidLatchOrder(network.name().to_owned()));
    }

    for node in &latch_order_list {
        if !network.node(*node)?.latch_output {
            let name = network.node(*node)?.name.clone();
            context.read_error(format!(
                "model `{}`: {name} is not the output of a latch",
                network.name()
            ));
            return Err(ReadError::InvalidLatchOrder(name));
        }
    }

    network.latch_order = latch_order_list;
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatchReport {
    pub patched_models: Vec<String>,
    pub remaining_models: Vec<String>,
}

pub fn patch_ready_models(
    context: &mut ReadContext,
    models: &mut ModelTable,
) -> ReadResult<PatchReport> {
    let mut patched_models = Vec::new();
    loop {
        let ready = models
            .models
            .iter()
            .find(|(_, entry)| entry.depends_on == 0)
            .map(|(name, _)| name.clone());
        let Some(name) = ready else {
            break;
        };

        let entry = models.models.remove(&name).expect("ready model exists");
        for patch in entry.patch_lists {
            if let Some(dependent) = models.models.get_mut(&patch.netname) {
                dependent.depends_on = dependent.depends_on.saturating_sub(1);
            }
        }
        patched_models.push(name);
    }

    let remaining_models = models.models.keys().cloned().collect::<Vec<_>>();
    if !remaining_models.is_empty() {
        context.read_error("Cyclic model dependency detected, probably involving:");
        for name in &remaining_models {
            context.read_error(name);
        }
        return Err(ReadError::NetworkCycle(remaining_models));
    }

    Ok(PatchReport {
        patched_models,
        remaining_models,
    })
}

pub fn dc_network_check(context: &mut ReadContext, network: &ReadNetwork) -> ReadResult<()> {
    let dc_network = network.dc_network().ok_or(ReadError::MissingDcNetwork)?;
    for output in dc_network.primary_outputs() {
        let name = &dc_network.node(*output)?.name;
        if network.find_node(name).is_none() {
            context.read_error(format!(
                "fatal: output {name} appears only in the external don't care network."
            ));
            return Err(ReadError::DcOutputOnly(name.clone()));
        }
    }

    for input in dc_network.primary_inputs() {
        let name = &dc_network.node(*input)?.name;
        if network.find_node(name).is_none() {
            context.read_error(format!(
                "fatal: input {name} appears only in the external don't care  network."
            ));
            return Err(ReadError::DcInputOnly(name.clone()));
        }
    }

    Ok(())
}

fn dereference_primary_output(network: &ReadNetwork, node: ReadNodeId) -> ReadResult<ReadNodeId> {
    if network.node(node)?.kind == ReadNodeKind::PrimaryOutput {
        return network
            .node(node)?
            .fanins
            .first()
            .copied()
            .ok_or(ReadError::MissingNode(node));
    }

    Ok(node)
}

fn compute_relax(network: &ReadNetwork, po_list: &[ReadNodeId]) -> (bool, bool) {
    let relax_i = !network
        .primary_inputs()
        .iter()
        .copied()
        .any(|node| network.node(node).is_ok_and(|node| node.is_real_pi));
    let relax_o = po_list.is_empty();
    (relax_i, relax_o)
}

fn read_input_node(
    context: &mut ReadContext,
    cursor: &mut TokenCursor,
    network: &ReadNetwork,
    errmsg: &str,
) -> ReadResult<ReadNodeId> {
    let word = require_token(cursor, errmsg)?;
    let node = network
        .find_node(&word)
        .ok_or_else(|| ReadError::MissingName(word.clone()))?;
    if network.node(node)?.kind != ReadNodeKind::PrimaryInput {
        context.read_error(format!(
            "node '{word}' not defined as input node in {errmsg}"
        ));
        return Err(ReadError::NotInputNode(word));
    }

    Ok(node)
}

fn read_output_node(
    context: &mut ReadContext,
    cursor: &mut TokenCursor,
    network: &ReadNetwork,
    po_list: &[ReadNodeId],
    errmsg: &str,
) -> ReadResult<ReadNodeId> {
    let word = require_token(cursor, errmsg)?;
    let output = po_list
        .iter()
        .copied()
        .find(|node| network.node(*node).is_ok_and(|node| node.name == word));
    match output {
        Some(node) => Ok(node),
        None => {
            context.read_error(format!(
                "node '{word}' not defined as output node in {errmsg}"
            ));
            Err(ReadError::NotOutputNode(word))
        }
    }
}

fn require_token(cursor: &mut TokenCursor, context: &str) -> ReadResult<String> {
    cursor
        .next_token()
        .ok_or_else(|| ReadError::MissingToken(context.to_owned()))
}

fn parse_token_number(cursor: &mut TokenCursor, context: &str) -> ReadResult<f64> {
    let token = require_token(cursor, context)?;
    parse_number(&token)
}

fn parse_one_number(value: &str) -> ReadResult<f64> {
    let mut words = value.split_whitespace();
    let Some(first) = words.next() else {
        return Err(ReadError::MissingToken("number".to_owned()));
    };
    if words.next().is_some() {
        return Err(ReadError::WrongArgumentCount("number".to_owned()));
    }

    parse_number(first)
}

fn parse_number(value: &str) -> ReadResult<f64> {
    value
        .parse::<f64>()
        .map_err(|_| ReadError::InvalidNumber(value.to_owned()))
}

fn madeup_name_index(name: &str) -> Option<usize> {
    let body = name.strip_prefix('[')?.strip_suffix(']')?;
    if body.is_empty() || !body.chars().all(|item| item.is_ascii_digit()) {
        return None;
    }

    body.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(network: &mut ReadNetwork, name: &str) -> ReadNodeId {
        network
            .add_node(ReadNode::new(name, ReadNodeKind::PrimaryInput))
            .unwrap()
    }

    #[test]
    fn context_formats_filename_and_line() {
        let mut context = ReadContext::new();
        context.register_filename(Some("demo.blif"));
        context.increment_line();
        context.read_error("bad token");

        assert_eq!(context.diagnostics(), ["\"demo.blif\", line 2: bad token"]);
    }

    #[test]
    fn token_cursor_preserves_rest_semantics() {
        let mut cursor = TokenCursor::new("a b c");

        assert_eq!(cursor.next_token().as_deref(), Some("a"));
        assert_eq!(cursor.rest(), Some("b c"));
        assert_eq!(cursor.next_token().as_deref(), Some("b"));
        assert_eq!(cursor.next_token().as_deref(), Some("c"));
        assert_eq!(cursor.next_token(), None);
    }

    #[test]
    fn find_or_create_dereferences_primary_output() {
        let mut network = ReadNetwork::new("n");
        let a = input(&mut network, "a");
        network.add_primary_output_named(a, "y").unwrap();

        assert_eq!(read_find_or_create_node(&mut network, "y").unwrap(), a);
        assert!(network.find_node("missing").is_none());
        assert!(read_find_or_create_node(&mut network, "missing").is_ok());
    }

    #[test]
    fn slif_complement_creates_negative_literal() {
        let mut network = ReadNetwork::new("n");

        let node = read_slif_find_or_create_node(&mut network, "foo'", false).unwrap();
        let positive = network.find_node("foo").unwrap();

        assert_eq!(
            network.node(node).unwrap().function,
            ReadNodeFunction::Literal {
                source: positive,
                phase: false,
            }
        );
    }

    #[test]
    fn slif_complement_flag_keeps_complement_name_as_positive_node() {
        let mut network = ReadNetwork::new("n");

        let complement = read_slif_find_or_create_node(&mut network, "foo'", true).unwrap();
        let positive = network.find_node("foo").unwrap();

        assert_eq!(network.node(complement).unwrap().name, "foo'");
        assert_eq!(
            network.node(positive).unwrap().function,
            ReadNodeFunction::Literal {
                source: complement,
                phase: false,
            }
        );
    }

    #[test]
    fn constants_are_replaced_on_lookup() {
        let mut network = ReadNetwork::new("n");

        let zero = read_slif_find_or_create_node(&mut network, "0", false).unwrap();
        let one = read_slif_find_or_create_node(&mut network, "1", false).unwrap();

        assert_eq!(
            network.node(zero).unwrap().function,
            ReadNodeFunction::Constant(false)
        );
        assert_eq!(
            network.node(one).unwrap().function,
            ReadNodeFunction::Constant(true)
        );
    }

    #[test]
    fn madeup_names_are_renamed_with_leading_space() {
        let mut network = ReadNetwork::new("n");
        let node = network
            .add_node(ReadNode::new("[42]", ReadNodeKind::Internal))
            .unwrap();

        assert!(read_change_madeup_name(&mut network, node).unwrap());
        assert_eq!(network.node(node).unwrap().name, " 42");
    }

    #[test]
    fn check_io_list_relaxes_missing_inputs_when_no_real_inputs_exist() {
        let mut context = ReadContext::new();
        let mut network = ReadNetwork::new("n");
        let floating = network
            .add_node(ReadNode::new("floating", ReadNodeKind::Internal))
            .unwrap();
        let mut po_list = Vec::new();

        read_check_io_list(&mut context, &mut network, &mut po_list, true).unwrap();

        assert_eq!(
            network.node(floating).unwrap().kind,
            ReadNodeKind::PrimaryInput
        );
        assert!(context.diagnostics().is_empty());
    }

    #[test]
    fn check_io_list_replaces_undriven_internal_with_zero_when_inputs_exist() {
        let mut context = ReadContext::new();
        let mut network = ReadNetwork::new("n");
        input(&mut network, "a");
        let floating = network
            .add_node(ReadNode::new("floating", ReadNodeKind::Internal))
            .unwrap();
        let mut po_list = vec![floating];

        read_check_io_list(&mut context, &mut network, &mut po_list, true).unwrap();

        assert_eq!(
            network.node(floating).unwrap().function,
            ReadNodeFunction::Constant(false)
        );
        assert_eq!(context.diagnostics().len(), 2);
    }

    #[test]
    fn check_io_list_relaxes_missing_outputs_when_po_list_empty() {
        let mut context = ReadContext::new();
        let mut network = ReadNetwork::new("n");
        let a = input(&mut network, "a");
        let mut po_list = Vec::new();

        read_check_io_list(&mut context, &mut network, &mut po_list, true).unwrap();

        assert_eq!(po_list, vec![a]);
    }

    #[test]
    fn hack_outputs_renames_input_output_collision() {
        let mut context = ReadContext::new();
        let mut network = ReadNetwork::new("n");
        let a = input(&mut network, "a");

        let outputs = read_hack_outputs(&mut context, &mut network, &[a]).unwrap();

        assert_eq!(outputs.len(), 1);
        assert_eq!(network.node(a).unwrap().name, "IN-a_0");
        assert_eq!(context.diagnostics().len(), 1);
    }

    #[test]
    fn cleanup_buffers_patches_primary_output_driver() {
        let mut network = ReadNetwork::new("n");
        let a = input(&mut network, "a");
        let buffer = network.add_node(ReadNode::buffer("buf", a)).unwrap();
        let output = network.add_primary_output_named(buffer, "y").unwrap();

        let removed = read_cleanup_buffers(&mut network).unwrap();

        assert_eq!(removed, vec![buffer]);
        assert_eq!(network.node(output).unwrap().fanins, vec![a]);
        assert!(network.node(buffer).is_err());
    }

    #[test]
    fn delay_area_and_defaults_are_recorded() {
        let mut context = ReadContext::new();
        let mut network = ReadNetwork::new("n");

        assert_eq!(
            read_delay(
                &mut context,
                &mut network,
                &[],
                "area",
                "3.5",
                SlifAttributeMode::NotSlif,
            )
            .unwrap(),
            DelayReadStatus::DelayRelated
        );
        read_delay(
            &mut context,
            &mut network,
            &[],
            "default_input_drive",
            "1.0 2.0",
            SlifAttributeMode::NotSlif,
        )
        .unwrap();

        assert_eq!(network.area(), Some(3.5));
        assert_eq!(
            network.default_delay(DelayParam::DefaultDriveRise),
            Some(1.0)
        );
        assert_eq!(
            network.default_delay(DelayParam::DefaultDriveFall),
            Some(2.0)
        );
    }

    #[test]
    fn delay_applies_to_named_input() {
        let mut context = ReadContext::new();
        let mut network = ReadNetwork::new("n");
        let a = input(&mut network, "a");

        read_delay(
            &mut context,
            &mut network,
            &[],
            "delay",
            "a NONINV 4.0 5.0 1.0 2.0 3.0 4.0",
            SlifAttributeMode::NotSlif,
        )
        .unwrap();

        let delays = &network.node(a).unwrap().delays;
        assert_eq!(delays.get(&DelayParam::InputLoad), Some(&4.0));
        assert_eq!(delays.get(&DelayParam::MaxInputLoad), Some(&5.0));
        assert_eq!(delays.get(&DelayParam::Phase), Some(&1.0));
    }

    #[test]
    fn output_required_uses_pending_po_list_names() {
        let mut context = ReadContext::new();
        let mut network = ReadNetwork::new("n");
        let a = input(&mut network, "a");
        let pending_output = network
            .add_node(ReadNode::new("y", ReadNodeKind::Internal))
            .unwrap();

        read_delay(
            &mut context,
            &mut network,
            &[pending_output],
            "output_required",
            "y 9.0 8.0",
            SlifAttributeMode::NotSlif,
        )
        .unwrap();

        assert_eq!(
            network
                .node(pending_output)
                .unwrap()
                .delays
                .get(&DelayParam::RequiredRise),
            Some(&9.0)
        );
        assert!(network.node(a).unwrap().delays.is_empty());
    }

    #[test]
    fn delay_cleanup_removes_latch_output_arrivals() {
        let mut network = ReadNetwork::new("n");
        let a = input(&mut network, "a");
        network.node_mut(a).unwrap().latch_output = true;
        network
            .node_mut(a)
            .unwrap()
            .delays
            .insert(DelayParam::ArrivalRise, 1.0);
        network
            .node_mut(a)
            .unwrap()
            .delays
            .insert(DelayParam::ArrivalFall, 2.0);
        network.add_primary_output_named(a, "y").unwrap();

        read_delay_cleanup(&mut network).unwrap();

        assert!(
            !network
                .node(a)
                .unwrap()
                .delays
                .contains_key(&DelayParam::ArrivalRise)
        );
        assert!(
            !network
                .node(a)
                .unwrap()
                .delays
                .contains_key(&DelayParam::ArrivalFall)
        );
    }

    #[test]
    fn model_table_reuses_existing_entry() {
        let mut models = ModelTable::default();
        read_find_or_create_model(&mut models, "m").depends_on = 2;

        assert_eq!(read_find_or_create_model(&mut models, "m").depends_on, 2);
        assert_eq!(models.len(), 1);
    }

    #[test]
    fn patch_ready_models_reduces_dependents_and_reports_cycles() {
        let mut context = ReadContext::new();
        let mut models = ModelTable::default();
        models
            .find_or_create_model("leaf")
            .patch_lists
            .push(PatchInfo {
                netname: "top".to_owned(),
                actuals: Vec::new(),
                formals: None,
                inputs: None,
            });
        models.find_or_create_model("top").depends_on = 1;

        let report = patch_ready_models(&mut context, &mut models).unwrap();

        assert_eq!(report.patched_models, vec!["leaf", "top"]);
        assert!(models.is_empty());

        models.find_or_create_model("a").depends_on = 1;
        models.find_or_create_model("b").depends_on = 1;
        let error = patch_ready_models(&mut context, &mut models).unwrap_err();
        assert!(matches!(error, ReadError::NetworkCycle(_)));
    }

    #[test]
    fn latch_order_requires_latch_outputs_when_stg_is_present() {
        let mut context = ReadContext::new();
        let mut network = ReadNetwork::new("n");
        network.set_stg_names_present(true);
        let a = input(&mut network, "a");

        let error = read_set_latch_order(&mut context, &mut network, vec![a]).unwrap_err();

        assert_eq!(error, ReadError::InvalidLatchOrder("a".to_owned()));
    }

    #[test]
    fn dc_network_check_rejects_orphan_dc_io() {
        let mut context = ReadContext::new();
        let mut care = ReadNetwork::new("care");
        input(&mut care, "a");
        let mut dc = ReadNetwork::new("dc");
        input(&mut dc, "missing");
        care.set_dc_network(Some(dc));

        let error = dc_network_check(&mut context, &care).unwrap_err();

        assert_eq!(error, ReadError::DcInputOnly("missing".to_owned()));
        assert_eq!(context.diagnostics().len(), 1);
    }

    #[test]
    fn filename_to_netname_uses_path_basename() {
        assert_eq!(filename_to_netname("dir/model.blif"), "model.blif");
        assert_eq!(filename_to_netname("model.eqn"), "model.eqn");
    }
}
