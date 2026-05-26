//! Native Rust helpers for writing SIS IO formats.
//!
//! The original `write_util.c` owns shared BLIF/SLIF writer behavior: deciding
//! which network artifacts should be printed, choosing the externally visible
//! node name, emitting `.names`/mapped gate records, wrapping long output lines,
//! and serializing delay annotations.  This port keeps those behaviors as safe
//! owned Rust APIs without process-global writer state.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::fmt::Write;

use crate::ports::network::network_util::{
    CoverValue, Network, NetworkUtilError, NodeId, NodeKind,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateBinding {
    pub gate_name: String,
    pub input_pin_names: Vec<String>,
    pub output_pin_name: String,
    pub latch_feedback_pin: Option<usize>,
}

impl GateBinding {
    pub fn new(
        gate_name: impl Into<String>,
        input_pin_names: impl Into<Vec<String>>,
        output_pin_name: impl Into<String>,
    ) -> Self {
        Self {
            gate_name: gate_name.into(),
            input_pin_names: input_pin_names.into(),
            output_pin_name: output_pin_name.into(),
            latch_feedback_pin: None,
        }
    }

    pub fn with_latch_feedback_pin(mut self, pin: usize) -> Self {
        self.latch_feedback_pin = Some(pin);
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayParameter {
    InputArrival,
    OutputRequired,
    InputDrive,
    OutputLoad,
    MaxInputLoad,
    WireLoadSlope,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: Option<f64>,
}

impl DelayTime {
    pub fn pair(rise: f64, fall: f64) -> Self {
        Self {
            rise,
            fall: Some(fall),
        }
    }

    pub fn scalar(value: f64) -> Self {
        Self {
            rise: value,
            fall: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClockTransition {
    Rise,
    Fall,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClockRelation {
    Before,
    After,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncEdge {
    pub relation: ClockRelation,
    pub transition: ClockTransition,
    pub clock: NodeId,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DelayTable {
    defaults: BTreeMap<DelayParameterKey, DelayTime>,
    node_delays: BTreeMap<(NodeId, DelayParameterKey), DelayTime>,
    sync_edges: BTreeMap<NodeId, SyncEdge>,
    blif_wire_loads: Vec<String>,
}

impl DelayTable {
    pub fn set_default(&mut self, parameter: DelayParameter, delay: DelayTime) {
        self.defaults.insert(parameter.into(), delay);
    }

    pub fn set_node_delay(&mut self, node: NodeId, parameter: DelayParameter, delay: DelayTime) {
        self.node_delays.insert((node, parameter.into()), delay);
    }

    pub fn set_sync_edge(&mut self, node: NodeId, edge: SyncEdge) {
        self.sync_edges.insert(node, edge);
    }

    pub fn add_blif_wire_load(&mut self, line: impl Into<String>) {
        self.blif_wire_loads.push(line.into());
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum DelayParameterKey {
    InputArrival,
    OutputRequired,
    InputDrive,
    OutputLoad,
    MaxInputLoad,
    WireLoadSlope,
}

impl From<DelayParameter> for DelayParameterKey {
    fn from(value: DelayParameter) -> Self {
        match value {
            DelayParameter::InputArrival => Self::InputArrival,
            DelayParameter::OutputRequired => Self::OutputRequired,
            DelayParameter::InputDrive => Self::InputDrive,
            DelayParameter::OutputLoad => Self::OutputLoad,
            DelayParameter::MaxInputLoad => Self::MaxInputLoad,
            DelayParameter::WireLoadSlope => Self::WireLoadSlope,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WriteContext<'a> {
    network: &'a Network,
    real_primary_inputs: BTreeSet<NodeId>,
    real_primary_outputs: BTreeSet<NodeId>,
    latch_output_for_latch_input: BTreeMap<NodeId, NodeId>,
    gates: BTreeMap<NodeId, GateBinding>,
    delays: DelayTable,
}

impl<'a> WriteContext<'a> {
    pub fn new(network: &'a Network) -> Self {
        Self {
            network,
            real_primary_inputs: network.primary_inputs().iter().copied().collect(),
            real_primary_outputs: network.primary_outputs().iter().copied().collect(),
            latch_output_for_latch_input: BTreeMap::new(),
            gates: BTreeMap::new(),
            delays: DelayTable::default(),
        }
    }

    pub fn network(&self) -> &Network {
        self.network
    }

    pub fn set_real_primary_inputs(&mut self, inputs: impl IntoIterator<Item = NodeId>) {
        self.real_primary_inputs = inputs.into_iter().collect();
    }

    pub fn set_real_primary_outputs(&mut self, outputs: impl IntoIterator<Item = NodeId>) {
        self.real_primary_outputs = outputs.into_iter().collect();
    }

    pub fn set_latch_output_for_latch_input(&mut self, latch_input: NodeId, latch_output: NodeId) {
        self.latch_output_for_latch_input
            .insert(latch_input, latch_output);
    }

    pub fn set_gate(&mut self, node: NodeId, gate: GateBinding) {
        self.gates.insert(node, gate);
    }

    pub fn delays_mut(&mut self) -> &mut DelayTable {
        &mut self.delays
    }

    fn is_real_primary_input(&self, node: NodeId) -> bool {
        self.real_primary_inputs.contains(&node)
    }

    fn is_real_primary_output(&self, node: NodeId) -> bool {
        self.real_primary_outputs.contains(&node)
    }

    fn latch_output_for_latch_input(&self, node: NodeId) -> Option<NodeId> {
        self.latch_output_for_latch_input.get(&node).copied()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WriteUtilError {
    Network(NetworkUtilError),
    MissingPrimaryOutputFanin(NodeId),
    MissingGateInputPin { node: NodeId, pin: usize },
}

impl fmt::Display for WriteUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(error) => write!(f, "{error}"),
            Self::MissingPrimaryOutputFanin(node) => {
                write!(f, "primary output {} has no driver", node.index())
            }
            Self::MissingGateInputPin { node, pin } => {
                write!(
                    f,
                    "node {} has no mapped gate input pin {pin}",
                    node.index()
                )
            }
        }
    }
}

impl Error for WriteUtilError {}

impl From<NetworkUtilError> for WriteUtilError {
    fn from(value: NetworkUtilError) -> Self {
        Self::Network(value)
    }
}

pub type WriteUtilResult<T> = Result<T, WriteUtilError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BreakWriter {
    output: String,
    break_string: String,
    break_column: usize,
    column: usize,
}

impl Default for BreakWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl BreakWriter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            break_string: String::new(),
            break_column: 32_000,
            column: 0,
        }
    }

    pub fn with_params(break_string: impl Into<String>, break_column: usize) -> Self {
        let mut writer = Self::new();
        writer.set_params(break_string, break_column);
        writer
    }

    pub fn set_params(&mut self, break_string: impl Into<String>, break_column: usize) {
        self.break_string = break_string.into();
        self.break_column = break_column;
        self.column = 0;
    }

    pub fn fputs(&mut self, value: &str) {
        if self.column + value.len() > self.break_column && !self.break_string.is_empty() {
            let break_string = self.break_string.clone();
            self.write_raw(&break_string);
        }

        self.write_raw(value);
    }

    pub fn fputc(&mut self, value: char) {
        self.output.push(value);
        self.column = if value == '\n' { 0 } else { self.column + 1 };
    }

    pub fn fprintf(&mut self, args: fmt::Arguments<'_>) {
        let mut buffer = String::new();
        buffer
            .write_fmt(args)
            .expect("formatting into String cannot fail");
        self.fputs(&buffer);
    }

    pub fn as_str(&self) -> &str {
        &self.output
    }

    pub fn into_string(self) -> String {
        self.output
    }

    fn write_raw(&mut self, value: &str) {
        for item in value.chars() {
            self.fputc(item);
        }
    }
}

pub fn io_po_fanout_count(context: &WriteContext<'_>, node: NodeId) -> WriteUtilResult<usize> {
    Ok(po_fanouts(context, node)?.len())
}

pub fn io_po_fanout_first(
    context: &WriteContext<'_>,
    node: NodeId,
) -> WriteUtilResult<Option<NodeId>> {
    Ok(po_fanouts(context, node)?.first().copied())
}

pub fn io_rpo_fanout_count(context: &WriteContext<'_>, node: NodeId) -> WriteUtilResult<usize> {
    Ok(real_po_fanouts(context, node)?.len())
}

pub fn io_rpo_fanout_first(
    context: &WriteContext<'_>,
    node: NodeId,
) -> WriteUtilResult<Option<NodeId>> {
    Ok(real_po_fanouts(context, node)?.first().copied())
}

pub fn io_lpo_fanout_count(context: &WriteContext<'_>, node: NodeId) -> WriteUtilResult<usize> {
    Ok(latch_input_fanouts(context, node)?.len())
}

pub fn io_lpo_fanout_first(
    context: &WriteContext<'_>,
    node: NodeId,
) -> WriteUtilResult<Option<NodeId>> {
    Ok(latch_input_fanouts(context, node)?.first().copied())
}

pub fn io_name(
    context: &WriteContext<'_>,
    node: NodeId,
    short_flag: bool,
) -> WriteUtilResult<String> {
    let mut printable = node;
    let mut kind = context.network.node(printable)?.kind;

    if kind == NodeKind::PrimaryOutput {
        if context.is_real_primary_output(printable) {
            return node_name(context, printable, short_flag);
        }

        printable = primary_output_driver(context.network, printable)?;
        kind = context.network.node(printable)?.kind;
    }

    if kind == NodeKind::PrimaryInput && context.is_real_primary_input(printable) {
        return node_name(context, printable, short_flag);
    }

    if let Some(real_output) = io_rpo_fanout_first(context, printable)? {
        if io_rpo_fanout_count(context, printable)? == 1 {
            printable = real_output;
        }
    }

    node_name(context, printable, short_flag)
}

pub fn io_node_name(context: &WriteContext<'_>, node: NodeId) -> WriteUtilResult<String> {
    io_name(context, node, false)
}

pub fn io_write_name(
    writer: &mut BreakWriter,
    context: &WriteContext<'_>,
    node: NodeId,
    short_flag: bool,
) -> WriteUtilResult<()> {
    writer.fputs(&io_name(context, node, short_flag)?);
    Ok(())
}

pub fn io_node_should_be_printed(
    context: &WriteContext<'_>,
    node: NodeId,
) -> WriteUtilResult<bool> {
    let network_node = context.network.node(node)?;
    if network_node.kind == NodeKind::PrimaryInput {
        return Ok(false);
    }

    if network_node.kind == NodeKind::PrimaryOutput {
        if !context.is_real_primary_output(node) {
            return Ok(false);
        }

        let fanin = primary_output_driver(context.network, node)?;
        let fanin_node = context.network.node(fanin)?;
        let real_po_count = io_rpo_fanout_count(context, fanin)?;
        if fanin_node.kind != NodeKind::PrimaryInput {
            if real_po_count == 1 {
                return Ok(false);
            }
        } else if real_po_count <= 1 && !context.is_real_primary_input(fanin) {
            return Ok(false);
        }
    }

    Ok(true)
}

pub fn io_write_node(
    writer: &mut BreakWriter,
    context: &WriteContext<'_>,
    node: NodeId,
    short_flag: bool,
) -> WriteUtilResult<()> {
    io_write_func(writer, context, node, short_flag, false)
}

pub fn io_write_gate(
    writer: &mut BreakWriter,
    context: &WriteContext<'_>,
    node: NodeId,
    short_flag: bool,
) -> WriteUtilResult<()> {
    io_write_func(writer, context, node, short_flag, true)
}

pub fn write_blif_slif_delay(
    writer: &mut BreakWriter,
    context: &WriteContext<'_>,
    slif: bool,
    short_flag: bool,
) -> WriteUtilResult<()> {
    let default_prefix = if slif {
        "global_attribute "
    } else {
        "default_"
    };
    let term = if slif { ';' } else { ' ' };

    if slif {
        writer.fputs(".type input_arrival %f %f ;\n");
        writer.fputs(".type output_required %f %f ;\n");
        writer.fputs(".type input_drive %f %f ;\n");
        writer.fputs(".type output_load %f ;\n");
    }

    write_default_pair(
        writer,
        context,
        default_prefix,
        "input_arrival",
        DelayParameterKey::InputArrival,
        term,
    );
    write_default_pair(
        writer,
        context,
        default_prefix,
        "output_required",
        DelayParameterKey::OutputRequired,
        term,
    );
    write_default_pair(
        writer,
        context,
        default_prefix,
        "input_drive",
        DelayParameterKey::InputDrive,
        term,
    );
    write_default_scalar(
        writer,
        context,
        default_prefix,
        "output_load",
        DelayParameterKey::OutputLoad,
        term,
    );
    write_default_scalar(
        writer,
        context,
        default_prefix,
        "max_input_load",
        DelayParameterKey::MaxInputLoad,
        term,
    );

    let wire_prefix = if slif { default_prefix } else { "" };
    write_default_scalar(
        writer,
        context,
        wire_prefix,
        "wire_load_slope",
        DelayParameterKey::WireLoadSlope,
        term,
    );

    if !slif {
        for line in &context.delays.blif_wire_loads {
            writer.fputs(line);
            if !line.ends_with('\n') {
                writer.fputc('\n');
            }
        }
    }

    let node_prefix = if slif { "attribute " } else { "" };
    for input in context.network.primary_inputs() {
        if !context.is_real_primary_input(*input) {
            continue;
        }

        write_node_pair(
            writer,
            context,
            *input,
            node_prefix,
            "input_arrival",
            DelayParameterKey::InputArrival,
            term,
            short_flag,
            !slif,
        )?;
        write_node_pair(
            writer,
            context,
            *input,
            node_prefix,
            "input_drive",
            DelayParameterKey::InputDrive,
            term,
            short_flag,
            false,
        )?;
        write_node_scalar(
            writer,
            context,
            *input,
            node_prefix,
            "max_input_load",
            DelayParameterKey::MaxInputLoad,
            term,
            short_flag,
        )?;
    }

    for output in context.network.primary_outputs() {
        if !context.is_real_primary_output(*output) {
            continue;
        }

        write_node_pair(
            writer,
            context,
            *output,
            node_prefix,
            "output_required",
            DelayParameterKey::OutputRequired,
            term,
            short_flag,
            !slif,
        )?;
        write_node_scalar(
            writer,
            context,
            *output,
            node_prefix,
            "output_load",
            DelayParameterKey::OutputLoad,
            term,
            short_flag,
        )?;
    }

    Ok(())
}

fn io_write_func(
    writer: &mut BreakWriter,
    context: &WriteContext<'_>,
    node: NodeId,
    short_flag: bool,
    mapped: bool,
) -> WriteUtilResult<()> {
    if !io_node_should_be_printed(context, node)? {
        return Ok(());
    }

    let gate = context.gates.get(&node);
    if !mapped || gate.is_none() {
        writer.fputs(".names");
        for fanin in &context.network.node(node)?.fanins {
            writer.fputc(' ');
            io_write_name(writer, context, *fanin, short_flag)?;
        }
        writer.fputc(' ');
        io_write_name(writer, context, node, short_flag)?;
        writer.fputc('\n');
        write_cover(writer, context, node)?;
        return Ok(());
    }

    let gate = gate.expect("gate checked above");
    let latch_output = io_lpo_fanout_first(context, node)?;
    if latch_output.is_some() {
        writer.fputs(".mlatch ");
    } else {
        writer.fputs(".gate ");
    }
    writer.fputs(&gate.gate_name);

    for (pin, fanin) in context.network.node(node)?.fanins.iter().enumerate() {
        if gate.latch_feedback_pin == Some(pin) {
            continue;
        }

        let pin_name = gate
            .input_pin_names
            .get(pin)
            .ok_or(WriteUtilError::MissingGateInputPin { node, pin })?;
        writer.fprintf(format_args!(
            " {pin_name}={}",
            io_name(context, *fanin, short_flag)?
        ));
    }

    let output_node = latch_output
        .and_then(|latch_input| context.latch_output_for_latch_input(latch_input))
        .unwrap_or(node);
    writer.fprintf(format_args!(
        " {}={}",
        gate.output_pin_name,
        io_name(context, output_node, short_flag)?
    ));
    Ok(())
}

fn write_cover(
    writer: &mut BreakWriter,
    context: &WriteContext<'_>,
    node: NodeId,
) -> WriteUtilResult<()> {
    let network_node = context.network.node(node)?;
    if network_node.kind == NodeKind::PrimaryOutput {
        writer.fputs("1 1\n");
        return Ok(());
    }

    if let Some(cover) = &network_node.cover {
        for (cube_index, cube) in cover.cubes().iter().enumerate() {
            if cube.values().len() != network_node.fanins.len() {
                return Err(NetworkUtilError::InvalidCover {
                    node,
                    cube: cube_index,
                }
                .into());
            }

            for value in cube.values() {
                writer.fputc(match value {
                    CoverValue::Zero => '0',
                    CoverValue::One => '1',
                    CoverValue::DontCare => '-',
                });
            }
            writer.fputs(" 1\n");
        }
    }

    Ok(())
}

fn write_default_pair(
    writer: &mut BreakWriter,
    context: &WriteContext<'_>,
    prefix: &str,
    name: &str,
    parameter: DelayParameterKey,
    term: char,
) {
    if let Some(delay) =
        get_default_delay_param(context, parameter).filter(|item| item.fall.is_some())
    {
        writer.fprintf(format_args!(
            ".{prefix}{name} {:4.2} {:4.2}{term}\n",
            delay.rise,
            delay.fall.unwrap_or(0.0)
        ));
    }
}

fn write_default_scalar(
    writer: &mut BreakWriter,
    context: &WriteContext<'_>,
    prefix: &str,
    name: &str,
    parameter: DelayParameterKey,
    term: char,
) {
    if let Some(delay) = get_default_delay_param(context, parameter) {
        writer.fprintf(format_args!(".{prefix}{name} {:4.2}{term}\n", delay.rise));
    }
}

#[allow(clippy::too_many_arguments)]
fn write_node_pair(
    writer: &mut BreakWriter,
    context: &WriteContext<'_>,
    node: NodeId,
    prefix: &str,
    name: &str,
    parameter: DelayParameterKey,
    term: char,
    short_flag: bool,
    include_sync_edge: bool,
) -> WriteUtilResult<()> {
    let Some(delay) = get_delay_param(context, node, parameter).filter(|item| item.fall.is_some())
    else {
        return Ok(());
    };
    writer.fprintf(format_args!(
        ".{prefix}{name} {} {:4.2} {:4.2}",
        io_name(context, node, short_flag)?,
        delay.rise,
        delay.fall.unwrap_or(0.0)
    ));
    if include_sync_edge {
        write_synch_edge(writer, context, node, short_flag)?;
    }
    writer.fputc(term);
    writer.fputc('\n');
    Ok(())
}

fn write_node_scalar(
    writer: &mut BreakWriter,
    context: &WriteContext<'_>,
    node: NodeId,
    prefix: &str,
    name: &str,
    parameter: DelayParameterKey,
    term: char,
    short_flag: bool,
) -> WriteUtilResult<()> {
    let Some(delay) = get_delay_param(context, node, parameter) else {
        return Ok(());
    };
    writer.fprintf(format_args!(
        ".{prefix}{name} {} {:4.2}{term}\n",
        io_name(context, node, short_flag)?,
        delay.rise
    ));
    Ok(())
}

fn write_synch_edge(
    writer: &mut BreakWriter,
    context: &WriteContext<'_>,
    node: NodeId,
    short_flag: bool,
) -> WriteUtilResult<()> {
    let Some(edge) = context.delays.sync_edges.get(&node) else {
        return Ok(());
    };
    let relation = match edge.relation {
        ClockRelation::Before => 'b',
        ClockRelation::After => 'a',
    };
    let transition = match edge.transition {
        ClockTransition::Rise => 'r',
        ClockTransition::Fall => 'f',
    };
    writer.fprintf(format_args!(
        " {relation} {transition}'{}",
        io_name(context, edge.clock, short_flag)?
    ));
    Ok(())
}

fn get_default_delay_param(
    context: &WriteContext<'_>,
    parameter: DelayParameterKey,
) -> Option<DelayTime> {
    context.delays.defaults.get(&parameter).copied()
}

fn get_delay_param(
    context: &WriteContext<'_>,
    node: NodeId,
    parameter: DelayParameterKey,
) -> Option<DelayTime> {
    context.delays.node_delays.get(&(node, parameter)).copied()
}

fn po_fanouts(context: &WriteContext<'_>, node: NodeId) -> WriteUtilResult<Vec<NodeId>> {
    let mut result = Vec::new();
    for fanout in &context.network.node(node)?.fanouts {
        if context.network.node(*fanout)?.kind == NodeKind::PrimaryOutput {
            result.push(*fanout);
        }
    }

    Ok(result)
}

fn real_po_fanouts(context: &WriteContext<'_>, node: NodeId) -> WriteUtilResult<Vec<NodeId>> {
    Ok(po_fanouts(context, node)?
        .into_iter()
        .filter(|fanout| context.is_real_primary_output(*fanout))
        .collect())
}

fn latch_input_fanouts(context: &WriteContext<'_>, node: NodeId) -> WriteUtilResult<Vec<NodeId>> {
    Ok(po_fanouts(context, node)?
        .into_iter()
        .filter(|fanout| context.latch_output_for_latch_input(*fanout).is_some())
        .collect())
}

fn primary_output_driver(network: &Network, output: NodeId) -> WriteUtilResult<NodeId> {
    network
        .node(output)?
        .fanins
        .first()
        .copied()
        .ok_or(WriteUtilError::MissingPrimaryOutputFanin(output))
}

fn node_name(
    context: &WriteContext<'_>,
    node: NodeId,
    short_flag: bool,
) -> WriteUtilResult<String> {
    let network_node = context.network.node(node)?;
    Ok(if short_flag {
        network_node.short_name.clone()
    } else {
        network_node.name.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::network::network_util::{Cube, NetworkNode, SopCover};

    fn cube(values: &[CoverValue]) -> Cube {
        Cube::new(values.to_vec())
    }

    fn simple_network() -> (Network, NodeId, NodeId, NodeId, NodeId) {
        let mut network = Network::new();
        let a = network
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let b = network
            .add_primary_input(NetworkNode::new("b", NodeKind::PrimaryInput))
            .unwrap();
        let n = network
            .add_internal(
                "n",
                vec![a, b],
                SopCover::new([
                    cube(&[CoverValue::One, CoverValue::Zero]),
                    cube(&[CoverValue::DontCare, CoverValue::One]),
                ]),
            )
            .unwrap();
        let y = network.add_primary_output(n).unwrap();
        (network, a, b, n, y)
    }

    #[test]
    fn break_writer_inserts_configured_break_string_before_long_write() {
        let mut writer = BreakWriter::with_params("\\\n", 5);

        writer.fputs("abc");
        writer.fputs("def");

        assert_eq!(writer.as_str(), "abc\\\ndef");
    }

    #[test]
    fn fanout_counters_distinguish_real_primary_outputs() {
        let (mut network, _, _, n, y) = simple_network();
        let second = network.add_primary_output(n).unwrap();
        let mut context = WriteContext::new(&network);
        context.set_real_primary_outputs([y]);

        assert_eq!(io_po_fanout_count(&context, n).unwrap(), 2);
        assert_eq!(io_rpo_fanout_count(&context, n).unwrap(), 1);
        assert_eq!(io_rpo_fanout_first(&context, n).unwrap(), Some(y));
        assert!(!context.is_real_primary_output(second));
    }

    #[test]
    fn io_name_prefers_unique_real_primary_output_fanout() {
        let (network, _, _, n, y) = simple_network();
        let context = WriteContext::new(&network);

        assert_eq!(
            io_name(&context, n, false).unwrap(),
            network.node(y).unwrap().name
        );
    }

    #[test]
    fn io_name_uses_driver_name_for_artifact_primary_output() {
        let (network, _, _, n, y) = simple_network();
        let mut context = WriteContext::new(&network);
        context.set_real_primary_outputs([]);

        assert_eq!(
            io_name(&context, y, false).unwrap(),
            network.node(n).unwrap().name
        );
    }

    #[test]
    fn should_not_print_primary_inputs_or_single_output_driver_artifact() {
        let (network, a, _, n, y) = simple_network();
        let context = WriteContext::new(&network);

        assert!(!io_node_should_be_printed(&context, a).unwrap());
        assert!(!io_node_should_be_printed(&context, y).unwrap());
        assert!(io_node_should_be_printed(&context, n).unwrap());
    }

    #[test]
    fn writes_names_cover_rows() {
        let (network, _, _, n, _) = simple_network();
        let context = WriteContext::new(&network);
        let mut writer = BreakWriter::new();

        io_write_node(&mut writer, &context, n, false).unwrap();

        assert_eq!(writer.as_str(), ".names a b n\n10 1\n-1 1\n");
    }

    #[test]
    fn writes_primary_output_buffer_cover_when_output_must_be_printed() {
        let (mut network, a, _, _, y) = simple_network();
        let extra_output = network.add_primary_output(a).unwrap();
        let context = WriteContext::new(&network);
        let mut writer = BreakWriter::new();

        assert!(io_node_should_be_printed(&context, extra_output).unwrap());
        io_write_node(&mut writer, &context, y, false).unwrap();

        assert_eq!(writer.as_str(), "");
    }

    #[test]
    fn writes_mapped_gate_and_skips_latch_feedback_pin() {
        let (mut network, _, _, n, _) = simple_network();
        let latch_input = network.add_primary_output(n).unwrap();
        let latch_output = network
            .add_primary_input(NetworkNode::new("q", NodeKind::PrimaryInput))
            .unwrap();
        let mut context = WriteContext::new(&network);
        context.set_latch_output_for_latch_input(latch_input, latch_output);
        context.set_gate(
            n,
            GateBinding::new("dff", vec!["d".into(), "clk".into()], "q").with_latch_feedback_pin(1),
        );
        let mut writer = BreakWriter::new();

        io_write_gate(&mut writer, &context, n, false).unwrap();

        assert_eq!(writer.as_str(), ".mlatch dff d=a q=q");
    }

    #[test]
    fn writes_blif_delay_defaults_node_delays_and_sync_edges() {
        let (network, a, _, _, y) = simple_network();
        let mut context = WriteContext::new(&network);
        context
            .delays_mut()
            .set_default(DelayParameter::InputArrival, DelayTime::pair(1.0, 2.0));
        context.delays_mut().set_node_delay(
            a,
            DelayParameter::InputDrive,
            DelayTime::pair(3.0, 4.0),
        );
        context
            .delays_mut()
            .set_node_delay(y, DelayParameter::OutputLoad, DelayTime::scalar(5.0));
        context.delays_mut().set_node_delay(
            y,
            DelayParameter::OutputRequired,
            DelayTime::pair(6.0, 7.0),
        );
        context.delays_mut().set_sync_edge(
            y,
            SyncEdge {
                relation: ClockRelation::Before,
                transition: ClockTransition::Rise,
                clock: a,
            },
        );
        let mut writer = BreakWriter::new();

        write_blif_slif_delay(&mut writer, &context, false, false).unwrap();

        assert!(
            writer
                .as_str()
                .contains(".default_input_arrival 1.00 2.00 \n")
        );
        assert!(writer.as_str().contains(".input_drive a 3.00 4.00 \n"));
        assert!(
            writer
                .as_str()
                .contains(".output_required n 6.00 7.00 b r'a \n")
        );
        assert!(writer.as_str().contains(".output_load n 5.00 \n"));
    }

    #[test]
    fn writes_slif_delay_type_declarations_and_semicolon_terms() {
        let (network, a, _, _, _) = simple_network();
        let mut context = WriteContext::new(&network);
        context.delays_mut().set_node_delay(
            a,
            DelayParameter::InputDrive,
            DelayTime::pair(3.0, 4.0),
        );
        let mut writer = BreakWriter::new();

        write_blif_slif_delay(&mut writer, &context, true, false).unwrap();

        assert!(writer.as_str().starts_with(".type input_arrival %f %f ;\n"));
        assert!(
            writer
                .as_str()
                .contains(".attribute input_drive a 3.00 4.00;\n")
        );
    }
}
