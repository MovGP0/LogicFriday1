//! Native SLIF writer.
//!
//! The original SIS writer is a SLIF-flavoured variant of the BLIF writer. This
//! port keeps the writer as an ordinary Rust API over the native network model
//! and accepts small metadata sidecars for SIS concepts that are not yet stored
//! directly on `Network`, such as latches, mapped gates, and delay attributes.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::ports::network::network_util::{
    CoverValue, Network, NetworkNode, NetworkUtilError, NodeId, NodeKind,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteSlifOptions {
    pub use_short_names: bool,
    pub use_netlist_calls: bool,
    pub include_delay_attributes: bool,
    pub break_column: usize,
}

impl Default for WriteSlifOptions {
    fn default() -> Self {
        Self {
            use_short_names: false,
            use_netlist_calls: false,
            include_delay_attributes: false,
            break_column: 78,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SlifMetadata {
    latch_outputs: BTreeSet<NodeId>,
    latch_inputs: BTreeSet<NodeId>,
    control_outputs: BTreeSet<NodeId>,
    latches: Vec<SlifLatch>,
    gates: BTreeMap<NodeId, SlifGateCall>,
    delays: SlifDelayAttributes,
}

impl SlifMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_latch(&mut self, latch: SlifLatch) {
        self.latch_outputs.insert(latch.output);
        self.latch_inputs.insert(latch.input);
        self.latches.push(latch);
    }

    pub fn mark_control_output(&mut self, node: NodeId) {
        self.control_outputs.insert(node);
    }

    pub fn set_gate(&mut self, node: NodeId, gate: SlifGateCall) {
        self.gates.insert(node, gate);
    }

    pub fn delays_mut(&mut self) -> &mut SlifDelayAttributes {
        &mut self.delays
    }

    pub fn delays(&self) -> &SlifDelayAttributes {
        &self.delays
    }

    pub fn is_latch_output(&self, node: NodeId) -> bool {
        self.latch_outputs.contains(&node)
    }

    pub fn is_latch_input(&self, node: NodeId) -> bool {
        self.latch_inputs.contains(&node)
    }

    pub fn is_control_output(&self, node: NodeId) -> bool {
        self.control_outputs.contains(&node)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlifLatch {
    pub output: NodeId,
    pub input: NodeId,
    pub control: Option<NodeId>,
    pub mapped_gate_node: Option<NodeId>,
}

impl SlifLatch {
    pub fn new(output: NodeId, input: NodeId, control: Option<NodeId>) -> Self {
        Self {
            output,
            input,
            control,
            mapped_gate_node: None,
        }
    }

    pub fn with_mapped_gate_node(mut self, node: NodeId) -> Self {
        self.mapped_gate_node = Some(node);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlifGateCall {
    pub model_name: String,
    pub instance_name: String,
    pub latch_pin: Option<usize>,
}

impl SlifGateCall {
    pub fn new(model_name: impl Into<String>) -> Self {
        let model_name = model_name.into();
        Self {
            instance_name: model_name.clone(),
            model_name,
            latch_pin: None,
        }
    }

    pub fn with_instance_name(mut self, instance_name: impl Into<String>) -> Self {
        self.instance_name = instance_name.into();
        self
    }

    pub fn with_latch_pin(mut self, latch_pin: usize) -> Self {
        self.latch_pin = Some(latch_pin);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlifRiseFall {
    pub rise: f64,
    pub fall: f64,
}

impl SlifRiseFall {
    pub fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SlifDelayAttributes {
    pub default_input_arrival: Option<SlifRiseFall>,
    pub default_output_required: Option<SlifRiseFall>,
    pub default_input_drive: Option<SlifRiseFall>,
    pub default_output_load: Option<f64>,
    pub default_max_input_load: Option<f64>,
    pub wire_load_slope: Option<f64>,
    input_arrivals: BTreeMap<NodeId, SlifRiseFall>,
    input_drives: BTreeMap<NodeId, SlifRiseFall>,
    max_input_loads: BTreeMap<NodeId, f64>,
    output_requireds: BTreeMap<NodeId, SlifRiseFall>,
    output_loads: BTreeMap<NodeId, f64>,
}

impl SlifDelayAttributes {
    pub fn set_input_arrival(&mut self, node: NodeId, delay: SlifRiseFall) {
        self.input_arrivals.insert(node, delay);
    }

    pub fn set_input_drive(&mut self, node: NodeId, delay: SlifRiseFall) {
        self.input_drives.insert(node, delay);
    }

    pub fn set_max_input_load(&mut self, node: NodeId, load: f64) {
        self.max_input_loads.insert(node, load);
    }

    pub fn set_output_required(&mut self, node: NodeId, delay: SlifRiseFall) {
        self.output_requireds.insert(node, delay);
    }

    pub fn set_output_load(&mut self, node: NodeId, load: f64) {
        self.output_loads.insert(node, load);
    }

    pub fn is_empty(&self) -> bool {
        self.default_input_arrival.is_none()
            && self.default_output_required.is_none()
            && self.default_input_drive.is_none()
            && self.default_output_load.is_none()
            && self.default_max_input_load.is_none()
            && self.wire_load_slope.is_none()
            && self.input_arrivals.is_empty()
            && self.input_drives.is_empty()
            && self.max_input_loads.is_empty()
            && self.output_requireds.is_empty()
            && self.output_loads.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WriteSlifError {
    Network(NetworkUtilError),
    InvalidPrimaryOutput(NodeId),
    InvalidCover { node: NodeId, cube: usize },
    MissingLatchInput(NodeId),
    MissingMappedLatchGate(NodeId),
    EmptyNodeName(NodeId),
}

impl fmt::Display for WriteSlifError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(error) => write!(f, "{error}"),
            Self::InvalidPrimaryOutput(node) => {
                write!(f, "primary output {} must have one fanin", node.index())
            }
            Self::InvalidCover { node, cube } => {
                write!(f, "node {} has invalid SOP cube {cube}", node.index())
            }
            Self::MissingLatchInput(node) => {
                write!(f, "latch input {} must have a driving node", node.index())
            }
            Self::MissingMappedLatchGate(node) => {
                write!(f, "mapped latch node {} has no gate metadata", node.index())
            }
            Self::EmptyNodeName(node) => write!(f, "node {} has an empty name", node.index()),
        }
    }
}

impl Error for WriteSlifError {}

impl From<NetworkUtilError> for WriteSlifError {
    fn from(value: NetworkUtilError) -> Self {
        Self::Network(value)
    }
}

pub type WriteSlifResult<T> = Result<T, WriteSlifError>;

pub fn write_slif(network: &Network, options: &WriteSlifOptions) -> WriteSlifResult<String> {
    write_slif_with_metadata(network, options, &SlifMetadata::default())
}

pub fn write_slif_with_metadata(
    network: &Network,
    options: &WriteSlifOptions,
    metadata: &SlifMetadata,
) -> WriteSlifResult<String> {
    let mut writer = SlifWriter::new(network, options, metadata);
    writer.write_network()?;
    Ok(writer.finish())
}

struct SlifWriter<'a> {
    network: &'a Network,
    options: &'a WriteSlifOptions,
    metadata: &'a SlifMetadata,
    output: String,
    column: usize,
}

impl<'a> SlifWriter<'a> {
    fn new(
        network: &'a Network,
        options: &'a WriteSlifOptions,
        metadata: &'a SlifMetadata,
    ) -> Self {
        Self {
            network,
            options,
            metadata,
            output: String::new(),
            column: 0,
        }
    }

    fn write_network(&mut self) -> WriteSlifResult<()> {
        self.write_text(".model ");
        self.write_text(self.network.name());
        self.write_text(";\n.inputs");
        for input in self.network.primary_inputs() {
            if self.is_real_pi(*input) {
                self.write_text(" ");
                self.write_node_name(*input)?;
            }
        }
        self.write_text(";\n.outputs");
        for output in self.network.primary_outputs() {
            if self.is_real_po(*output) {
                self.write_text(" ");
                self.write_node_name(*output)?;
            }
        }
        self.write_text(";\n");

        if self.options.include_delay_attributes && !self.metadata.delays().is_empty() {
            self.write_delay_attributes()?;
        }

        let mut ignore_nodes = BTreeSet::new();
        if !self.options.use_short_names {
            for (node_id, _) in self.network.nodes() {
                if self.should_ignore_slif_node(node_id)? {
                    ignore_nodes.insert(node_id);
                }
            }
        }

        self.write_latches()?;
        for (node_id, _) in self.network.nodes() {
            if !ignore_nodes.contains(&node_id) {
                self.write_node(node_id, None)?;
            }
        }

        self.write_text(".endmodel ");
        self.write_text(self.network.name());
        self.write_text(";\n");
        Ok(())
    }

    fn finish(self) -> String {
        self.output
    }

    fn write_latches(&mut self) -> WriteSlifResult<()> {
        for latch in &self.metadata.latches {
            if self.options.use_netlist_calls {
                if let Some(mapped_node) = latch.mapped_gate_node {
                    self.write_node(mapped_node, Some(latch.input))?;
                    continue;
                }
            }

            self.write_node_name(latch.output)?;
            self.write_text(" = @D(");
            self.write_node_name(latch.input)?;
            self.write_text(", ");
            if let Some(control) = latch.control {
                self.write_node_name(control)?;
            } else {
                self.write_text("NIL");
            }
            self.write_text(");\n");
        }
        Ok(())
    }

    fn write_node(&mut self, node_id: NodeId, latch_input: Option<NodeId>) -> WriteSlifResult<()> {
        if !self.node_should_be_printed(node_id)? {
            return Ok(());
        }

        let gate = self.metadata.gates.get(&node_id);
        if !self.options.use_netlist_calls || gate.is_none() {
            return self.write_sop(node_id);
        }

        let gate = gate.expect("checked above");
        self.write_text(".call ");
        self.write_text(&gate.model_name);
        self.write_text(" ");
        self.write_text(&gate.instance_name);
        self.write_text(" (");

        let node = self.network.node(node_id)?;
        let mut first = true;
        for (index, fanin) in node.fanins.iter().copied().enumerate() {
            if gate.latch_pin == Some(index) {
                continue;
            }

            if !first {
                self.write_text(", ");
            }
            self.write_node_name(fanin)?;
            first = false;
        }

        let output_node = if let Some(latch_input) = latch_input {
            self.latch_output_for_input(latch_input)?
        } else {
            node_id
        };
        self.write_text(" ; ; ");
        self.write_node_name(output_node)?;
        self.write_text(");\n");
        Ok(())
    }

    fn write_sop(&mut self, node_id: NodeId) -> WriteSlifResult<()> {
        if !self.node_should_be_printed(node_id)? {
            return Ok(());
        }

        self.write_node_name(node_id)?;
        self.write_text(" = ");
        let node = self.network.node(node_id)?;

        match node.kind {
            NodeKind::PrimaryOutput => {
                let fanin = single_fanin(node_id, node)?;
                self.write_node_name(fanin)?;
                self.write_text(";\n");
                return Ok(());
            }
            NodeKind::PrimaryInput => return Ok(()),
            NodeKind::Internal | NodeKind::Unassigned => {}
        }

        let Some(cover) = &node.cover else {
            self.write_text(
                expression_text(node, self.network, self.options.use_short_names)?.as_str(),
            );
            self.write_text(";\n");
            return Ok(());
        };

        if cover.cubes().is_empty() {
            self.write_text("0;\n");
            return Ok(());
        }

        let mut first_cube = true;
        for (cube_index, cube) in cover.cubes().iter().enumerate().rev() {
            if cube.values().len() != node.fanins.len() {
                return Err(WriteSlifError::InvalidCover {
                    node: node_id,
                    cube: cube_index,
                });
            }

            if !first_cube {
                self.write_text(" + ");
            }

            let mut first_lit = true;
            for (literal_index, value) in cube.values().iter().enumerate() {
                match value {
                    CoverValue::Zero | CoverValue::One => {
                        if !first_lit {
                            self.write_text(" ");
                        }
                        self.write_node_name(node.fanins[literal_index])?;
                        if *value == CoverValue::Zero {
                            self.write_text("'");
                        }
                        first_lit = false;
                    }
                    CoverValue::DontCare => {}
                }
            }

            if first_lit {
                self.write_text("1");
            }
            first_cube = false;
        }

        self.write_text(";\n");
        Ok(())
    }

    fn write_delay_attributes(&mut self) -> WriteSlifResult<()> {
        let delays = self.metadata.delays();
        self.write_text(".type input_arrival %f %f ;\n");
        self.write_text(".type output_required %f %f ;\n");
        self.write_text(".type input_drive %f %f ;\n");
        self.write_text(".type output_load %f ;\n");

        if let Some(delay) = delays.default_input_arrival {
            self.write_formatted_line(format_args!(
                ".global_attribute input_arrival {:.2} {:.2};\n",
                delay.rise, delay.fall
            ));
        }
        if let Some(delay) = delays.default_output_required {
            self.write_formatted_line(format_args!(
                ".global_attribute output_required {:.2} {:.2};\n",
                delay.rise, delay.fall
            ));
        }
        if let Some(delay) = delays.default_input_drive {
            self.write_formatted_line(format_args!(
                ".global_attribute input_drive {:.2} {:.2};\n",
                delay.rise, delay.fall
            ));
        }
        if let Some(load) = delays.default_output_load {
            self.write_formatted_line(format_args!(".global_attribute output_load {:.2};\n", load));
        }
        if let Some(load) = delays.default_max_input_load {
            self.write_formatted_line(format_args!(
                ".global_attribute max_input_load {:.2};\n",
                load
            ));
        }
        if let Some(slope) = delays.wire_load_slope {
            self.write_formatted_line(format_args!(".wire_load_slope {:.2};\n", slope));
        }

        for input in self.network.primary_inputs() {
            if !self.is_real_pi(*input) {
                continue;
            }

            if let Some(delay) = delays.input_arrivals.get(input) {
                self.write_named_rise_fall(".attribute input_arrival", *input, *delay)?;
            }
            if let Some(delay) = delays.input_drives.get(input) {
                self.write_named_rise_fall(".attribute input_drive", *input, *delay)?;
            }
            if let Some(load) = delays.max_input_loads.get(input) {
                self.write_named_scalar(".attribute max_input_load", *input, *load)?;
            }
        }

        for output in self.network.primary_outputs() {
            if !self.is_real_po(*output) {
                continue;
            }

            if let Some(delay) = delays.output_requireds.get(output) {
                self.write_named_rise_fall(".attribute output_required", *output, *delay)?;
            }
            if let Some(load) = delays.output_loads.get(output) {
                self.write_named_scalar(".attribute output_load", *output, *load)?;
            }
        }

        Ok(())
    }

    fn write_named_rise_fall(
        &mut self,
        prefix: &str,
        node: NodeId,
        delay: SlifRiseFall,
    ) -> WriteSlifResult<()> {
        self.write_text(prefix);
        self.write_text(" ");
        self.write_node_name(node)?;
        self.write_formatted_line(format_args!(" {:.2} {:.2};\n", delay.rise, delay.fall));
        Ok(())
    }

    fn write_named_scalar(
        &mut self,
        prefix: &str,
        node: NodeId,
        value: f64,
    ) -> WriteSlifResult<()> {
        self.write_text(prefix);
        self.write_text(" ");
        self.write_node_name(node)?;
        self.write_formatted_line(format_args!(" {:.2};\n", value));
        Ok(())
    }

    fn write_node_name(&mut self, node: NodeId) -> WriteSlifResult<()> {
        let name = self.io_name(node)?;
        self.write_text(name.as_str());
        Ok(())
    }

    fn io_name(&self, node_id: NodeId) -> WriteSlifResult<String> {
        let mut selected = node_id;
        let mut node = self.network.node(selected)?;

        if node.kind == NodeKind::PrimaryOutput {
            if !self.is_real_po(selected) {
                selected = single_fanin(selected, node)?;
                node = self.network.node(selected)?;
            }
        }

        if node.kind != NodeKind::PrimaryInput || !self.is_real_pi(selected) {
            if let Some(real_output) = self.single_real_po_fanout(selected)? {
                selected = real_output;
            }
        }

        let selected_node = self.network.node(selected)?;
        let name = if self.options.use_short_names {
            &selected_node.short_name
        } else {
            &selected_node.name
        };

        if name.is_empty() {
            return Err(WriteSlifError::EmptyNodeName(selected));
        }

        Ok(name.clone())
    }

    fn node_should_be_printed(&self, node_id: NodeId) -> WriteSlifResult<bool> {
        let node = self.network.node(node_id)?;
        if node.kind == NodeKind::PrimaryInput {
            return Ok(false);
        }

        if node.kind == NodeKind::PrimaryOutput {
            if !self.is_real_po(node_id) {
                return Ok(false);
            }

            let fanin = single_fanin(node_id, node)?;
            let fanin_node = self.network.node(fanin)?;
            let real_po_count = self.real_po_fanout_count(fanin)?;
            if fanin_node.kind != NodeKind::PrimaryInput {
                if real_po_count == 1 {
                    return Ok(false);
                }
            } else if real_po_count <= 1 && !self.is_real_pi(fanin) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn should_ignore_slif_node(&mut self, node_id: NodeId) -> WriteSlifResult<bool> {
        let node = self.network.node(node_id)?;
        if node.kind != NodeKind::Internal {
            return Ok(false);
        }

        if is_inverter(node) {
            let fanin = single_fanin(node_id, node)?;
            let node_name = self.network.node(node_id)?.name.as_str();
            let fanin_name = self.network.node(fanin)?.name.as_str();
            if names_differ_by_trailing_quote(node_name, fanin_name) {
                return Ok(true);
            }
        }

        if cover_is_constant(node, false) && node.name == "0" {
            return Ok(true);
        }
        if cover_is_constant(node, true) && node.name == "1" {
            return Ok(true);
        }

        Ok(false)
    }

    fn is_real_pi(&self, node: NodeId) -> bool {
        !self.metadata.is_latch_output(node)
    }

    fn is_real_po(&self, node: NodeId) -> bool {
        !self.metadata.is_latch_input(node) && !self.metadata.is_control_output(node)
    }

    fn single_real_po_fanout(&self, node: NodeId) -> WriteSlifResult<Option<NodeId>> {
        let mut real_output = None;
        for fanout in &self.network.node(node)?.fanouts {
            if self.network.node(*fanout)?.kind == NodeKind::PrimaryOutput
                && self.is_real_po(*fanout)
            {
                if real_output.is_some() {
                    return Ok(None);
                }
                real_output = Some(*fanout);
            }
        }
        Ok(real_output)
    }

    fn real_po_fanout_count(&self, node: NodeId) -> WriteSlifResult<usize> {
        let mut count = 0;
        for fanout in &self.network.node(node)?.fanouts {
            if self.network.node(*fanout)?.kind == NodeKind::PrimaryOutput
                && self.is_real_po(*fanout)
            {
                count += 1;
            }
        }
        Ok(count)
    }

    fn latch_output_for_input(&self, input: NodeId) -> WriteSlifResult<NodeId> {
        self.metadata
            .latches
            .iter()
            .find(|latch| latch.input == input)
            .map(|latch| latch.output)
            .ok_or(WriteSlifError::MissingLatchInput(input))
    }

    fn write_text(&mut self, text: &str) {
        if self.options.break_column > 0
            && self.column + text.len() > self.options.break_column
            && self.column != 0
        {
            self.output.push('\n');
            self.column = 0;
        }

        for ch in text.chars() {
            self.output.push(ch);
            if ch == '\n' {
                self.column = 0;
            } else {
                self.column += 1;
            }
        }
    }

    fn write_formatted_line(&mut self, args: fmt::Arguments<'_>) {
        self.write_text(format!("{args}").as_str());
    }
}

fn single_fanin(node_id: NodeId, node: &NetworkNode) -> WriteSlifResult<NodeId> {
    node.fanins
        .first()
        .copied()
        .filter(|_| node.fanins.len() == 1)
        .ok_or(WriteSlifError::InvalidPrimaryOutput(node_id))
}

fn is_inverter(node: &NetworkNode) -> bool {
    node.fanins.len() == 1
        && node.cover.as_ref().is_some_and(|cover| {
            cover.cubes().len() == 1 && cover.cubes()[0].values() == [CoverValue::Zero].as_slice()
        })
}

fn cover_is_constant(node: &NetworkNode, value: bool) -> bool {
    let Some(cover) = &node.cover else {
        return false;
    };

    if !value {
        return cover.cubes().is_empty();
    }

    cover.cubes().len() == 1
        && cover.cubes()[0]
            .values()
            .iter()
            .all(|literal| *literal == CoverValue::DontCare)
}

fn names_differ_by_trailing_quote(left: &str, right: &str) -> bool {
    left.strip_suffix('\'') == Some(right) || right.strip_suffix('\'') == Some(left)
}

fn expression_text(
    node: &NetworkNode,
    network: &Network,
    use_short_names: bool,
) -> WriteSlifResult<String> {
    let Some(expression) = &node.expression else {
        return Ok("0".to_string());
    };
    render_expression(expression, network, use_short_names)
}

fn render_expression(
    expression: &crate::ports::network::network_util::BoolExpr,
    network: &Network,
    use_short_names: bool,
) -> WriteSlifResult<String> {
    match expression {
        crate::ports::network::network_util::BoolExpr::Constant(value) => {
            Ok(if *value { "1" } else { "0" }.to_string())
        }
        crate::ports::network::network_util::BoolExpr::Literal { node, phase } => {
            let network_node = network.node(*node)?;
            let name = if use_short_names {
                &network_node.short_name
            } else {
                &network_node.name
            };
            if *phase {
                Ok(name.clone())
            } else {
                Ok(format!("{name}'"))
            }
        }
        crate::ports::network::network_util::BoolExpr::Not(inner) => Ok(format!(
            "({})'",
            render_expression(inner, network, use_short_names)?
        )),
        crate::ports::network::network_util::BoolExpr::And(parts) => {
            render_expression_list(parts, " ", network, use_short_names)
        }
        crate::ports::network::network_util::BoolExpr::Or(parts) => {
            render_expression_list(parts, " + ", network, use_short_names)
        }
    }
}

fn render_expression_list(
    parts: &[crate::ports::network::network_util::BoolExpr],
    separator: &str,
    network: &Network,
    use_short_names: bool,
) -> WriteSlifResult<String> {
    let mut text = String::new();
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            text.push_str(separator);
        }
        text.push_str(render_expression(part, network, use_short_names)?.as_str());
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::network::network_util::{BoolExpr, Cube, SopCover};

    fn cube(values: &[CoverValue]) -> Cube {
        Cube::new(values.to_vec())
    }

    fn named_input(network: &mut Network, name: &str) -> NodeId {
        network
            .add_primary_input(NetworkNode::new(name, NodeKind::PrimaryInput))
            .unwrap()
    }

    fn named_output(network: &mut Network, fanin: NodeId, name: &str) -> NodeId {
        let mut node = NetworkNode::new(name, NodeKind::PrimaryOutput);
        node.short_name = format!("{name}_s");
        node.fanins.push(fanin);
        network.add_node(node).unwrap()
    }

    #[test]
    fn writes_slif_headers_outputs_and_sop_nodes() {
        let mut network = Network::new();
        network.set_name("demo");
        let a = named_input(&mut network, "a");
        let b = named_input(&mut network, "b");
        let n = network
            .add_internal(
                "n",
                [a, b],
                SopCover::new([
                    cube(&[CoverValue::One, CoverValue::Zero]),
                    cube(&[CoverValue::DontCare, CoverValue::One]),
                ]),
            )
            .unwrap();
        named_output(&mut network, n, "y");

        let text = write_slif(&network, &WriteSlifOptions::default()).unwrap();

        assert_eq!(
            text,
            ".model demo;\n.inputs a b;\n.outputs y;\ny = b + a b';\n.endmodel demo;\n"
        );
    }

    #[test]
    fn prints_primary_output_when_driver_feeds_multiple_outputs() {
        let mut network = Network::new();
        network.set_name("multi");
        let a = named_input(&mut network, "a");
        let n = network
            .add_internal("n", [a], SopCover::new([cube(&[CoverValue::One])]))
            .unwrap();
        named_output(&mut network, n, "y0");
        named_output(&mut network, n, "y1");

        let text = write_slif(&network, &WriteSlifOptions::default()).unwrap();

        assert!(text.contains("y0 = n;\n"));
        assert!(text.contains("y1 = n;\n"));
    }

    #[test]
    fn writes_slif_sop_syntax_for_internal_without_single_output_alias() {
        let mut network = Network::new();
        network.set_name("sop");
        let a = named_input(&mut network, "a");
        let b = named_input(&mut network, "b");
        let n = network
            .add_internal(
                "n",
                [a, b],
                SopCover::new([
                    cube(&[CoverValue::One, CoverValue::Zero]),
                    cube(&[CoverValue::DontCare, CoverValue::One]),
                ]),
            )
            .unwrap();
        named_output(&mut network, n, "y0");
        named_output(&mut network, n, "y1");

        let text = write_slif(&network, &WriteSlifOptions::default()).unwrap();

        assert!(text.contains("n = b + a b';\n"));
    }

    #[test]
    fn omits_latch_outputs_from_inputs_and_latch_inputs_from_outputs() {
        let mut network = Network::new();
        network.set_name("seq");
        let d = named_input(&mut network, "d");
        let q = named_input(&mut network, "q");
        let latch_input = named_output(&mut network, d, "q_next");
        let _y = named_output(&mut network, q, "y");

        let mut metadata = SlifMetadata::new();
        metadata.add_latch(SlifLatch::new(q, latch_input, None));

        let text =
            write_slif_with_metadata(&network, &WriteSlifOptions::default(), &metadata).unwrap();

        assert!(text.contains(".inputs d;"));
        assert!(text.contains(".outputs y;"));
        assert!(text.contains("y = @D(d, NIL);\n"), "{text}");
        assert!(!text.contains("q_next;"));
        assert!(!text.contains("y = q;\n"));
    }

    #[test]
    fn writes_mapped_gate_call_when_netlist_requested() {
        let mut network = Network::new();
        network.set_name("mapped");
        let a = named_input(&mut network, "a");
        let b = named_input(&mut network, "b");
        let n = network
            .add_internal(
                "n",
                [a, b],
                SopCover::new([cube(&[CoverValue::One, CoverValue::One])]),
            )
            .unwrap();
        named_output(&mut network, n, "z");
        named_output(&mut network, n, "tap");

        let mut metadata = SlifMetadata::new();
        metadata.set_gate(n, SlifGateCall::new("and2").with_instance_name("u1"));
        let options = WriteSlifOptions {
            use_netlist_calls: true,
            ..WriteSlifOptions::default()
        };

        let text = write_slif_with_metadata(&network, &options, &metadata).unwrap();

        assert!(text.contains(".call and2 u1 (a, b ; ; n);\n"));
    }

    #[test]
    fn ignores_quote_pair_inverter_and_named_constants() {
        let mut network = Network::new();
        network.set_name("ignore");
        let a = named_input(&mut network, "a");
        let inv = network
            .add_internal("a'", [a], SopCover::new([cube(&[CoverValue::Zero])]))
            .unwrap();
        let zero = network
            .add_internal("0", [], SopCover::new(Vec::<Cube>::new()))
            .unwrap();
        named_output(&mut network, inv, "y0");
        named_output(&mut network, zero, "y1");

        let text = write_slif(&network, &WriteSlifOptions::default()).unwrap();

        assert!(!text.contains("a' = "));
        assert!(!text.contains("0 = 0;"));
    }

    #[test]
    fn short_names_disable_slif_inverter_omission() {
        let mut network = Network::new();
        network.set_name("short");
        let a = named_input(&mut network, "a");
        let inv = network
            .add_internal("a'", [a], SopCover::new([cube(&[CoverValue::Zero])]))
            .unwrap();
        network.change_node_short_name(inv, "i").unwrap();
        named_output(&mut network, inv, "y0");
        named_output(&mut network, inv, "y1");

        let options = WriteSlifOptions {
            use_short_names: true,
            ..WriteSlifOptions::default()
        };
        let text = write_slif(&network, &options).unwrap();

        assert!(text.contains("i = a';\n"));
    }

    #[test]
    fn writes_delay_attributes_in_slif_form() {
        let mut network = Network::new();
        network.set_name("delay");
        let a = named_input(&mut network, "a");
        let y = named_output(&mut network, a, "y");

        let mut metadata = SlifMetadata::new();
        metadata.delays_mut().default_input_arrival = Some(SlifRiseFall::new(1.0, 2.5));
        metadata.delays_mut().set_output_load(y, 3.25);
        let options = WriteSlifOptions {
            include_delay_attributes: true,
            ..WriteSlifOptions::default()
        };

        let text = write_slif_with_metadata(&network, &options, &metadata).unwrap();

        assert!(text.contains(".type input_arrival %f %f ;\n"));
        assert!(text.contains(".global_attribute input_arrival 1.00 2.50;\n"));
        assert!(text.contains(".attribute output_load y 3.25;\n"));
    }

    #[test]
    fn renders_expression_nodes_when_no_cover_exists() {
        let mut network = Network::new();
        network.set_name("expr");
        let a = named_input(&mut network, "a");
        let b = named_input(&mut network, "b");
        let expr = BoolExpr::literal(a, true).and(BoolExpr::literal(b, false));
        let n = network.add_expression_node("n", expr).unwrap();
        named_output(&mut network, n, "y0");
        named_output(&mut network, n, "y1");

        let text = write_slif(&network, &WriteSlifOptions::default()).unwrap();

        assert!(text.contains("n = a b';\n"));
    }
}
