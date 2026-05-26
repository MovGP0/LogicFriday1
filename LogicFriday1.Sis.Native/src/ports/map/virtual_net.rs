//! Native Rust virtual mapped-network support for `sis/map/virtual_net.c`.
//!
//! The C implementation stores virtual fanin bindings in `MAP(node)->save_binding`
//! and reverse gate links in `MAP(node)->gate_link`. This module keeps the same
//! model as owned Rust data so mapper output can be formatted in the shape used
//! by SIS `print_gate` and `print_level` consumers.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::{self, Write};

pub const MINUS_INFINITY: DelayTime = DelayTime {
    rise: f64::NEG_INFINITY,
    fall: f64::NEG_INFINITY,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }

    pub fn min(self, other: Self) -> Self {
        Self {
            rise: self.rise.min(other.rise),
            fall: self.fall.min(other.fall),
        }
    }
}

impl Default for DelayTime {
    fn default() -> Self {
        MINUS_INFINITY
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NodeId(usize);

impl NodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GateKind {
    Inverter,
    Nand,
    Nor,
    Xor,
    Xnor,
    Mux,
    And,
    Or,
    One,
    Zero,
    Wire,
    Library(String),
}

impl GateKind {
    pub fn mnemonic(&self) -> &str {
        match self {
            Self::Inverter => "inv",
            Self::Nand => "nand",
            Self::Nor => "nor",
            Self::Xor => "exo",
            Self::Xnor => "exn",
            Self::Mux => "mux",
            Self::And => "and",
            Self::Or => "or",
            Self::One => "one",
            Self::Zero => "zer",
            Self::Wire => "wire",
            Self::Library(name) => name.as_str(),
        }
    }

    pub fn is_wire(&self) -> bool {
        matches!(self, Self::Wire)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceRef {
    Node(NodeId),
    ConstantZero,
    ConstantOne,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GateLinkKey {
    pub node: NodeId,
    pub pin: isize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GateLink {
    pub node: NodeId,
    pub pin: isize,
    pub load: f64,
    pub slack: f64,
    pub required: DelayTime,
}

impl GateLink {
    pub fn new(node: NodeId, pin: isize) -> Self {
        Self {
            node,
            pin,
            load: 0.0,
            slack: 0.0,
            required: MINUS_INFINITY,
        }
    }

    fn key(self) -> GateLinkKey {
        GateLinkKey {
            node: self.node,
            pin: self.pin,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualMappedNode {
    pub name: String,
    pub kind: NodeKind,
    pub gate: Option<GateKind>,
    pub save_binding: Vec<SourceRef>,
    pub load: f64,
    pub required: DelayTime,
    gate_links: BTreeMap<GateLinkKey, GateLink>,
}

impl VirtualMappedNode {
    fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            gate: None,
            save_binding: Vec::new(),
            load: 0.0,
            required: MINUS_INFINITY,
            gate_links: BTreeMap::new(),
        }
    }

    pub fn gate_links(&self) -> impl Iterator<Item = &GateLink> {
        self.gate_links.values()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VirtualNetworkError {
    MissingNode(NodeId),
    InvalidPrimaryOutputFanin(NodeId),
    InvalidGateFanin { node: NodeId, pin: usize },
    CannotRemoveExternalNode(NodeId),
    CycleDetected,
}

impl fmt::Display for VirtualNetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(f, "missing virtual network node {}", node.index()),
            Self::InvalidPrimaryOutputFanin(node) => {
                write!(f, "primary output {} must have one fanin", node.index())
            }
            Self::InvalidGateFanin { node, pin } => {
                write!(f, "node {} has invalid fanin pin {pin}", node.index())
            }
            Self::CannotRemoveExternalNode(node) => {
                write!(f, "cannot remove external node {}", node.index())
            }
            Self::CycleDetected => write!(f, "virtual network contains a cycle"),
        }
    }
}

impl std::error::Error for VirtualNetworkError {}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VirtualMappedNetwork {
    nodes: Vec<VirtualMappedNode>,
    inputs: Vec<NodeId>,
    outputs: Vec<NodeId>,
}

impl VirtualMappedNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_primary_input(&mut self, name: impl Into<String>) -> NodeId {
        let id = self.push_node(VirtualMappedNode::new(name, NodeKind::PrimaryInput));
        self.inputs.push(id);
        id
    }

    pub fn add_primary_output(
        &mut self,
        name: impl Into<String>,
        fanin: SourceRef,
    ) -> Result<NodeId, VirtualNetworkError> {
        let id = self.push_node(VirtualMappedNode::new(name, NodeKind::PrimaryOutput));
        self.outputs.push(id);
        self.set_primary_output_fanin(id, fanin)?;
        Ok(id)
    }

    pub fn add_gate(
        &mut self,
        name: impl Into<String>,
        gate: GateKind,
        fanins: Vec<SourceRef>,
    ) -> NodeId {
        let mut node = VirtualMappedNode::new(name, NodeKind::Internal);
        node.gate = Some(gate);
        node.save_binding = fanins;
        self.push_node(node)
    }

    pub fn node(&self, id: NodeId) -> Option<&VirtualMappedNode> {
        self.nodes.get(id.index())
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut VirtualMappedNode> {
        self.nodes.get_mut(id.index())
    }

    pub fn nodes(&self) -> &[VirtualMappedNode] {
        &self.nodes
    }

    pub fn inputs(&self) -> &[NodeId] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[NodeId] {
        &self.outputs
    }

    pub fn setup_gate_links(&mut self) -> Result<(), VirtualNetworkError> {
        for node in &mut self.nodes {
            node.gate_links.clear();
        }

        for id in 0..self.nodes.len() {
            let node_id = NodeId(id);
            match self.nodes[id].kind {
                NodeKind::PrimaryInput => {}
                NodeKind::PrimaryOutput => {
                    let source = self.primary_output_fanin(node_id)?;
                    self.add_to_gate_link(source, GateLink::new(node_id, -1))?;
                }
                NodeKind::Internal if self.nodes[id].gate.is_some() => {
                    for pin in 0..self.nodes[id].save_binding.len() {
                        let source = self.nodes[id].save_binding[pin];
                        self.add_to_gate_link(source, GateLink::new(node_id, pin as isize))?;
                    }
                }
                NodeKind::Internal => {}
            }
        }

        Ok(())
    }

    pub fn add_to_gate_link(
        &mut self,
        source: SourceRef,
        link: GateLink,
    ) -> Result<(), VirtualNetworkError> {
        self.set_link_binding(source, link)?;

        if let SourceRef::Node(source) = source {
            let node = self
                .node_mut(source)
                .ok_or(VirtualNetworkError::MissingNode(source))?;
            node.gate_links.insert(link.key(), link);
        }

        Ok(())
    }

    pub fn gate_link(&self, source: NodeId, node: NodeId, pin: isize) -> Option<&GateLink> {
        self.node(source)?
            .gate_links
            .get(&GateLinkKey { node, pin })
    }

    pub fn remove_gate_link(
        &mut self,
        source: NodeId,
        node: NodeId,
        pin: isize,
    ) -> Option<GateLink> {
        self.node_mut(source)?
            .gate_links
            .remove(&GateLinkKey { node, pin })
    }

    pub fn remove_node(
        &mut self,
        node: NodeId,
        recursive: bool,
    ) -> Result<(), VirtualNetworkError> {
        let kind = self
            .node(node)
            .ok_or(VirtualNetworkError::MissingNode(node))?
            .kind;

        if kind != NodeKind::Internal {
            return Err(VirtualNetworkError::CannotRemoveExternalNode(node));
        }

        if self
            .node(node)
            .and_then(|item| item.gate.as_ref())
            .is_none()
        {
            return Ok(());
        }

        let fanins = self.nodes[node.index()].save_binding.clone();
        for (pin, source) in fanins.into_iter().enumerate() {
            if let SourceRef::Node(source) = source {
                self.remove_gate_link(source, node, pin as isize);
                if recursive
                    && self
                        .node(source)
                        .is_some_and(|source_node| source_node.gate_links.is_empty())
                {
                    let _ = self.remove_node(source, true);
                }
            }
        }

        let mapped_node = &mut self.nodes[node.index()];
        mapped_node.load = 0.0;
        mapped_node.required = MINUS_INFINITY;
        mapped_node.gate = None;
        mapped_node.save_binding.clear();
        mapped_node.gate_links.clear();

        Ok(())
    }

    pub fn remove_wires(&mut self) -> Result<(), VirtualNetworkError> {
        let wire_nodes = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                node.gate
                    .as_ref()
                    .is_some_and(GateKind::is_wire)
                    .then_some(NodeId(index))
            })
            .collect::<Vec<_>>();

        for wire in wire_nodes {
            let Some(source) = self.nodes[wire.index()].save_binding.first().copied() else {
                continue;
            };
            let links = self.nodes[wire.index()]
                .gate_links
                .values()
                .copied()
                .collect::<Vec<_>>();

            for link in links {
                self.add_to_gate_link(source, link)?;
            }

            self.remove_node(wire, false)?;
        }

        Ok(())
    }

    pub fn update_link_required_times(
        &mut self,
        node: NodeId,
        required: &[DelayTime],
    ) -> Result<(), VirtualNetworkError> {
        let kind = self
            .node(node)
            .ok_or(VirtualNetworkError::MissingNode(node))?
            .kind;

        match kind {
            NodeKind::PrimaryInput => {}
            NodeKind::PrimaryOutput => {
                let source = self.primary_output_fanin(node)?;
                if let SourceRef::Node(source) = source {
                    let node_required = self.nodes[node.index()].required;
                    if let Some(link) = self.nodes.get_mut(source.index()).and_then(|source_node| {
                        source_node
                            .gate_links
                            .get_mut(&GateLinkKey { node, pin: -1 })
                    }) {
                        link.required = node_required;
                    }
                }
            }
            NodeKind::Internal => {
                for (pin, pin_required) in required.iter().enumerate() {
                    let Some(source) = self.nodes[node.index()].save_binding.get(pin).copied()
                    else {
                        return Err(VirtualNetworkError::InvalidGateFanin { node, pin });
                    };

                    if let SourceRef::Node(source) = source {
                        if let Some(link) =
                            self.nodes.get_mut(source.index()).and_then(|source_node| {
                                source_node.gate_links.get_mut(&GateLinkKey {
                                    node,
                                    pin: pin as isize,
                                })
                            })
                        {
                            link.required = *pin_required;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn compute_load(&self, node: NodeId, wire_load: impl Fn(usize) -> f64) -> f64 {
        let Some(node) = self.node(node) else {
            return 0.0;
        };
        let load = node.gate_links.values().map(|link| link.load).sum::<f64>();
        load + wire_load(node.gate_links.len())
    }

    pub fn compute_min_required(&self, node: NodeId) -> Option<DelayTime> {
        self.node(node)?
            .gate_links
            .values()
            .map(|link| link.required)
            .reduce(DelayTime::min)
    }

    pub fn format_print_gate(&self) -> Result<String, VirtualNetworkError> {
        let mut output = String::new();
        self.write_print_gate(&mut output)
            .expect("writing to a String should not fail");
        Ok(output)
    }

    pub fn write_print_gate(&self, writer: &mut impl Write) -> Result<(), VirtualNetworkError> {
        let mapped = self.mapped_output_nodes()?;
        writeln!(writer, "nodes={}", mapped.len()).expect("writing to a String should not fail");

        let indexes = self.print_gate_indexes(&mapped);
        for node in mapped {
            let mapped_node = self
                .node(node)
                .ok_or(VirtualNetworkError::MissingNode(node))?;
            let gate = mapped_node
                .gate
                .as_ref()
                .ok_or(VirtualNetworkError::MissingNode(node))?;

            if self.output_names_for_driver(node).is_empty() {
                write!(
                    writer,
                    "[{}] {} {}",
                    indexes[&node],
                    gate.mnemonic(),
                    mapped_node.save_binding.len()
                )
                .expect("writing to a String should not fail");
            } else {
                write!(
                    writer,
                    "{{{}}} {} {}",
                    self.output_names_for_driver(node).join(","),
                    gate.mnemonic(),
                    mapped_node.save_binding.len()
                )
                .expect("writing to a String should not fail");
            }

            for (pin, source) in mapped_node.save_binding.iter().enumerate() {
                write!(
                    writer,
                    " pin{}={}",
                    pin,
                    self.format_source_for_print_gate(*source, &indexes)?
                )
                .expect("writing to a String should not fail");
            }
            writer
                .write_char('\n')
                .expect("writing to a String should not fail");
        }

        Ok(())
    }

    pub fn format_print_level_summary(&self) -> Result<String, VirtualNetworkError> {
        Ok(format!("{}\n", self.level_count()?))
    }

    pub fn format_print_level(&self) -> Result<String, VirtualNetworkError> {
        let mut output = String::new();
        self.write_print_level(&mut output)
            .expect("writing to a String should not fail");
        Ok(output)
    }

    pub fn write_print_level(&self, writer: &mut impl Write) -> Result<(), VirtualNetworkError> {
        let levels = self.levels()?;
        writeln!(
            writer,
            "Total number of levels = {}",
            levels.len().saturating_sub(1)
        )
        .expect("writing to a String should not fail");

        for (level, nodes) in levels.iter().enumerate() {
            if level == 0 {
                self.write_level_line(writer, self.inputs.iter().copied())?;
            } else {
                self.write_level_line(writer, nodes.iter().copied())?;
            }
        }

        Ok(())
    }

    pub fn level_count(&self) -> Result<usize, VirtualNetworkError> {
        Ok(self.levels()?.len().saturating_sub(1))
    }

    pub fn levels(&self) -> Result<Vec<Vec<NodeId>>, VirtualNetworkError> {
        let mut indegrees = vec![0usize; self.nodes.len()];
        let mut fanouts = vec![Vec::<NodeId>::new(); self.nodes.len()];

        for (index, node) in self.nodes.iter().enumerate() {
            if node.kind == NodeKind::Internal && node.gate.is_none() {
                continue;
            }

            for source in &node.save_binding {
                if let SourceRef::Node(source) = source {
                    fanouts[source.index()].push(NodeId(index));
                    indegrees[index] += 1;
                }
            }
        }

        let mut queue = indegrees
            .iter()
            .enumerate()
            .filter_map(|(index, indegree)| (*indegree == 0).then_some(NodeId(index)))
            .collect::<VecDeque<_>>();
        let mut node_level = vec![0usize; self.nodes.len()];
        let mut visited = 0usize;

        while let Some(node) = queue.pop_front() {
            visited += 1;
            for fanout in &fanouts[node.index()] {
                node_level[fanout.index()] =
                    node_level[fanout.index()].max(node_level[node.index()] + 1);
                indegrees[fanout.index()] -= 1;
                if indegrees[fanout.index()] == 0 {
                    queue.push_back(*fanout);
                }
            }
        }

        if visited != self.nodes.len() {
            return Err(VirtualNetworkError::CycleDetected);
        }

        let max_level = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.kind == NodeKind::Internal && node.gate.is_some())
            .map(|(index, _)| node_level[index])
            .max()
            .unwrap_or(0);
        let mut levels = vec![Vec::new(); max_level + 1];

        for (index, node) in self.nodes.iter().enumerate() {
            if node.kind == NodeKind::Internal && node.gate.is_some() {
                levels[node_level[index]].push(NodeId(index));
            }
        }

        if levels.is_empty() {
            levels.push(Vec::new());
        }

        Ok(levels)
    }

    fn push_node(&mut self, node: VirtualMappedNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    fn primary_output_fanin(&self, output: NodeId) -> Result<SourceRef, VirtualNetworkError> {
        let node = self
            .node(output)
            .ok_or(VirtualNetworkError::MissingNode(output))?;

        if node.kind != NodeKind::PrimaryOutput || node.save_binding.len() != 1 {
            return Err(VirtualNetworkError::InvalidPrimaryOutputFanin(output));
        }

        Ok(node.save_binding[0])
    }

    fn set_primary_output_fanin(
        &mut self,
        output: NodeId,
        fanin: SourceRef,
    ) -> Result<(), VirtualNetworkError> {
        let node = self
            .node_mut(output)
            .ok_or(VirtualNetworkError::MissingNode(output))?;
        if node.kind != NodeKind::PrimaryOutput {
            return Err(VirtualNetworkError::InvalidPrimaryOutputFanin(output));
        }
        node.save_binding = vec![fanin];
        Ok(())
    }

    fn set_link_binding(
        &mut self,
        source: SourceRef,
        link: GateLink,
    ) -> Result<(), VirtualNetworkError> {
        let linked_node = self
            .node_mut(link.node)
            .ok_or(VirtualNetworkError::MissingNode(link.node))?;

        if linked_node.kind == NodeKind::PrimaryOutput {
            linked_node.save_binding = vec![source];
            return Ok(());
        }

        if link.pin < 0 {
            return Err(VirtualNetworkError::InvalidGateFanin {
                node: link.node,
                pin: 0,
            });
        }

        let pin = link.pin as usize;
        if pin >= linked_node.save_binding.len() {
            return Err(VirtualNetworkError::InvalidGateFanin {
                node: link.node,
                pin,
            });
        }

        linked_node.save_binding[pin] = source;
        Ok(())
    }

    fn mapped_output_nodes(&self) -> Result<Vec<NodeId>, VirtualNetworkError> {
        let mut result = Vec::new();
        let mut seen = BTreeSet::new();

        for (index, node) in self.nodes.iter().enumerate() {
            if node.kind == NodeKind::Internal && node.gate.is_some() {
                let id = NodeId(index);
                if seen.insert(id) {
                    result.push(id);
                }
            }
        }

        for output in &self.outputs {
            if let SourceRef::Node(source) = self.primary_output_fanin(*output)? {
                if self
                    .node(source)
                    .is_some_and(|node| node.kind == NodeKind::Internal && node.gate.is_some())
                    && seen.insert(source)
                {
                    result.push(source);
                }
            }
        }

        Ok(result)
    }

    fn print_gate_indexes(&self, mapped: &[NodeId]) -> BTreeMap<NodeId, usize> {
        mapped
            .iter()
            .enumerate()
            .map(|(index, node)| (*node, index))
            .collect()
    }

    fn output_names_for_driver(&self, driver: NodeId) -> Vec<String> {
        self.outputs
            .iter()
            .filter_map(|output| {
                (self.primary_output_fanin(*output).ok()? == SourceRef::Node(driver))
                    .then(|| self.nodes[output.index()].name.clone())
            })
            .collect()
    }

    fn format_source_for_print_gate(
        &self,
        source: SourceRef,
        indexes: &BTreeMap<NodeId, usize>,
    ) -> Result<String, VirtualNetworkError> {
        match source {
            SourceRef::ConstantZero => Ok("{0}".to_owned()),
            SourceRef::ConstantOne => Ok("{1}".to_owned()),
            SourceRef::Node(node) => {
                let mapped_node = self
                    .node(node)
                    .ok_or(VirtualNetworkError::MissingNode(node))?;
                if mapped_node.kind == NodeKind::PrimaryInput {
                    Ok(mapped_node.name.clone())
                } else if let Some(index) = indexes.get(&node) {
                    Ok(format!("[{index}]"))
                } else {
                    Ok(mapped_node.name.clone())
                }
            }
        }
    }

    fn write_level_line(
        &self,
        writer: &mut impl Write,
        nodes: impl IntoIterator<Item = NodeId>,
    ) -> Result<(), VirtualNetworkError> {
        for node in nodes {
            let item = self
                .node(node)
                .ok_or(VirtualNetworkError::MissingNode(node))?;
            if item.kind == NodeKind::Internal {
                write!(writer, "[{}] ", node.index()).expect("writing to a String should not fail");
            } else {
                write!(writer, "{{{}}} ", item.name).expect("writing to a String should not fail");
            }
        }
        writer
            .write_char('\n')
            .expect("writing to a String should not fail");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_gate_links_builds_reverse_pin_bindings() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let y_gate = network.add_gate(
            "n1",
            GateKind::Nand,
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );
        let y = network
            .add_primary_output("y", SourceRef::Node(y_gate))
            .unwrap();

        network.setup_gate_links().unwrap();

        assert_eq!(network.gate_link(a, y_gate, 0).unwrap().node, y_gate);
        assert_eq!(network.gate_link(b, y_gate, 1).unwrap().pin, 1);
        assert_eq!(network.gate_link(y_gate, y, -1).unwrap().node, y);
    }

    #[test]
    fn remove_wires_moves_sinks_to_wire_source() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let wire = network.add_gate("w", GateKind::Wire, vec![SourceRef::Node(a)]);
        let inv = network.add_gate("i", GateKind::Inverter, vec![SourceRef::Node(wire)]);
        network
            .add_primary_output("y", SourceRef::Node(inv))
            .unwrap();
        network.setup_gate_links().unwrap();

        network.remove_wires().unwrap();

        assert_eq!(
            network.node(inv).unwrap().save_binding,
            vec![SourceRef::Node(a)]
        );
        assert!(network.node(wire).unwrap().gate.is_none());
        assert!(network.gate_link(a, inv, 0).is_some());
    }

    #[test]
    fn required_times_update_existing_gate_links() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let gate = network.add_gate("n1", GateKind::Inverter, vec![SourceRef::Node(a)]);
        network.setup_gate_links().unwrap();

        network
            .update_link_required_times(gate, &[DelayTime::new(1.0, 2.0)])
            .unwrap();

        assert_eq!(
            network.gate_link(a, gate, 0).unwrap().required,
            DelayTime::new(1.0, 2.0)
        );
    }

    #[test]
    fn print_gate_uses_logic_friday_parseable_shape() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let n1 = network.add_gate(
            "n1",
            GateKind::Nand,
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );
        let n2 = network.add_gate("n2", GateKind::Inverter, vec![SourceRef::Node(n1)]);
        network
            .add_primary_output("f", SourceRef::Node(n2))
            .unwrap();

        assert_eq!(
            network.format_print_gate().unwrap(),
            "nodes=2\n[0] nand 2 pin0=a pin1=b\n{f} inv 1 pin0=[0]\n"
        );
    }

    #[test]
    fn print_level_emits_summary_and_level_rows() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let n1 = network.add_gate(
            "n1",
            GateKind::And,
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );
        let n2 = network.add_gate("n2", GateKind::Or, vec![SourceRef::Node(n1)]);
        network
            .add_primary_output("f", SourceRef::Node(n2))
            .unwrap();

        assert_eq!(network.format_print_level_summary().unwrap(), "2\n");
        assert_eq!(
            network.format_print_level().unwrap(),
            "Total number of levels = 2\n{a} {b} \n[2] \n[3] \n"
        );
    }
}
