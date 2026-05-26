//! Native Rust value accounting for SIS factored-form nodes.
//!
//! This module models the literal counting, fanin-use counting, node-value
//! scoring, printing, and ordering helpers from the SIS factor value routines on
//! owned Rust data. It intentionally has no per-file C ABI entry points.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub const INFINITY_VALUE: i32 = 1_000_000_000;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactorTree {
    Leaf { fanin_index: usize },
    And(Vec<FactorTree>),
    Or(Vec<FactorTree>),
}

impl FactorTree {
    pub fn leaf(fanin_index: usize) -> Self {
        Self::Leaf { fanin_index }
    }

    pub fn and(children: impl IntoIterator<Item = FactorTree>) -> Self {
        Self::And(children.into_iter().collect())
    }

    pub fn or(children: impl IntoIterator<Item = FactorTree>) -> Self {
        Self::Or(children.into_iter().collect())
    }

    pub fn literal_count(&self) -> usize {
        match self {
            Self::Leaf { .. } => 1,
            Self::And(children) | Self::Or(children) => {
                children.iter().map(Self::literal_count).sum()
            }
        }
    }

    pub fn use_count(&self, fanin_index: usize) -> usize {
        match self {
            Self::Leaf {
                fanin_index: leaf_index,
            } => usize::from(*leaf_index == fanin_index),
            Self::And(children) | Self::Or(children) => children
                .iter()
                .map(|child| child.use_count(fanin_index))
                .sum(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorValueNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub fanouts: Vec<NodeId>,
    pub factor: Option<FactorTree>,
}

impl FactorValueNode {
    pub fn new(id: usize, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            factor: None,
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = NodeId>) -> Self {
        self.fanins = fanins.into_iter().collect();
        self
    }

    pub fn with_fanouts(mut self, fanouts: impl IntoIterator<Item = NodeId>) -> Self {
        self.fanouts = fanouts.into_iter().collect();
        self
    }

    pub fn with_factor(mut self, factor: FactorTree) -> Self {
        self.factor = Some(factor);
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FactorValueNetwork {
    nodes: HashMap<NodeId, FactorValueNode>,
}

impl FactorValueNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: FactorValueNode) {
        self.nodes.insert(node.id, node);
    }

    pub fn node(&self, id: NodeId) -> FactorValueResult<&FactorValueNode> {
        self.nodes.get(&id).ok_or(FactorValueError::UnknownNode(id))
    }

    pub fn node_value(&self, node: NodeId) -> FactorValueResult<i32> {
        node_value(self, node)
    }

    pub fn factor_num_used(&self, output: NodeId, input: NodeId) -> FactorValueResult<usize> {
        factor_num_used(self, output, input)
    }

    pub fn factor_num_literal(&self, node: NodeId) -> FactorValueResult<usize> {
        factor_num_literal(self, node)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactorValueError {
    UnknownNode(NodeId),
    FailsIntegerRange { value: usize },
    MissingFanin { output: NodeId, input: NodeId },
}

impl fmt::Display for FactorValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown factor node {}", node.0),
            Self::FailsIntegerRange { value } => {
                write!(f, "factor value {value} does not fit in a signed integer")
            }
            Self::MissingFanin { output, input } => {
                write!(f, "node {} is not a fanin of node {}", input.0, output.0)
            }
        }
    }
}

impl Error for FactorValueError {}

pub type FactorValueResult<T> = Result<T, FactorValueError>;

pub fn node_value(network: &FactorValueNetwork, node: NodeId) -> FactorValueResult<i32> {
    let node_ref = network.node(node)?;
    if matches!(
        node_ref.kind,
        NodeKind::PrimaryInput | NodeKind::PrimaryOutput
    ) {
        return Ok(INFINITY_VALUE);
    }

    if node_ref.fanouts.is_empty() {
        return Ok(INFINITY_VALUE);
    }

    let mut is_primary_output = false;
    let mut num_used = 0_usize;
    for fanout in &node_ref.fanouts {
        let fanout_ref = network.node(*fanout)?;
        if fanout_ref.kind == NodeKind::PrimaryOutput {
            is_primary_output = true;
        } else {
            num_used += factor_num_used(network, *fanout, node)?;
        }
    }

    let num_lit = factor_num_literal(network, node)?;
    let num_used_i32 = to_i32(num_used)?;
    let num_lit_i32 = to_i32(num_lit)?;
    let mut value = num_used_i32 * num_lit_i32 - num_used_i32 - num_lit_i32;

    if is_primary_output {
        value += num_lit_i32;
    }

    Ok(value)
}

pub fn factor_num_used(
    network: &FactorValueNetwork,
    output: NodeId,
    input: NodeId,
) -> FactorValueResult<usize> {
    let output_ref = network.node(output)?;
    let fanin_index = output_ref
        .fanins
        .iter()
        .position(|fanin| *fanin == input)
        .ok_or(FactorValueError::MissingFanin { output, input })?;

    Ok(output_ref
        .factor
        .as_ref()
        .map_or(0, |factor| factor.use_count(fanin_index)))
}

pub fn factor_num_literal(network: &FactorValueNetwork, node: NodeId) -> FactorValueResult<usize> {
    let node_ref = network.node(node)?;
    if matches!(
        node_ref.kind,
        NodeKind::PrimaryInput | NodeKind::PrimaryOutput
    ) {
        return Ok(0);
    }

    Ok(node_ref
        .factor
        .as_ref()
        .map_or(0, FactorTree::literal_count))
}

pub fn value_line(network: &FactorValueNetwork, node: NodeId) -> FactorValueResult<String> {
    let node_ref = network.node(node)?;
    let value = node_value(network, node)?;
    if value >= INFINITY_VALUE {
        Ok(format!("{}:\t(inf)\n", node_ref.name))
    } else {
        Ok(format!("{}:\t{}\n", node_ref.name, value))
    }
}

pub fn compare_value_increasing(
    network: &FactorValueNetwork,
    left: NodeId,
    right: NodeId,
) -> FactorValueResult<Ordering> {
    Ok(node_value(network, left)?.cmp(&node_value(network, right)?))
}

pub fn compare_value_decreasing(
    network: &FactorValueNetwork,
    left: NodeId,
    right: NodeId,
) -> FactorValueResult<Ordering> {
    Ok(node_value(network, right)?.cmp(&node_value(network, left)?))
}

pub fn sort_by_value_increasing(
    network: &FactorValueNetwork,
    nodes: &mut [NodeId],
) -> FactorValueResult<()> {
    sort_by_value(network, nodes, false)
}

pub fn sort_by_value_decreasing(
    network: &FactorValueNetwork,
    nodes: &mut [NodeId],
) -> FactorValueResult<()> {
    sort_by_value(network, nodes, true)
}

fn sort_by_value(
    network: &FactorValueNetwork,
    nodes: &mut [NodeId],
    decreasing: bool,
) -> FactorValueResult<()> {
    let mut keyed = nodes
        .iter()
        .copied()
        .map(|node| Ok((node, node_value(network, node)?)))
        .collect::<FactorValueResult<Vec<_>>>()?;

    if decreasing {
        keyed.sort_by(|left, right| right.1.cmp(&left.1));
    } else {
        keyed.sort_by(|left, right| left.1.cmp(&right.1));
    }

    for (slot, (node, _)) in nodes.iter_mut().zip(keyed) {
        *slot = node;
    }

    Ok(())
}

fn to_i32(value: usize) -> FactorValueResult<i32> {
    i32::try_from(value).map_err(|_| FactorValueError::FailsIntegerRange { value })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(index: usize) -> FactorTree {
        FactorTree::leaf(index)
    }

    fn sample_network() -> FactorValueNetwork {
        let a = NodeId(1);
        let b = NodeId(2);
        let f = NodeId(3);
        let g = NodeId(4);
        let y = NodeId(5);

        let mut network = FactorValueNetwork::new();
        network.add_node(FactorValueNode::new(a.0, "a", NodeKind::PrimaryInput));
        network.add_node(FactorValueNode::new(b.0, "b", NodeKind::PrimaryInput));
        network.add_node(
            FactorValueNode::new(f.0, "f", NodeKind::Internal)
                .with_fanins([a, b])
                .with_fanouts([g, y])
                .with_factor(FactorTree::and([leaf(0), leaf(1), leaf(1)])),
        );
        network.add_node(
            FactorValueNode::new(g.0, "g", NodeKind::Internal)
                .with_fanins([f, b])
                .with_fanouts([y])
                .with_factor(FactorTree::or([leaf(0), leaf(1), leaf(0)])),
        );
        network
            .add_node(FactorValueNode::new(y.0, "y", NodeKind::PrimaryOutput).with_fanins([f, g]));

        network
    }

    #[test]
    fn primary_nodes_have_infinite_value_and_zero_literals() {
        let mut network = FactorValueNetwork::new();
        let input = NodeId(1);
        let output = NodeId(2);
        network.add_node(FactorValueNode::new(input.0, "a", NodeKind::PrimaryInput));
        network.add_node(FactorValueNode::new(output.0, "y", NodeKind::PrimaryOutput));

        assert_eq!(node_value(&network, input).unwrap(), INFINITY_VALUE);
        assert_eq!(node_value(&network, output).unwrap(), INFINITY_VALUE);
        assert_eq!(factor_num_literal(&network, input).unwrap(), 0);
        assert_eq!(factor_num_literal(&network, output).unwrap(), 0);
    }

    #[test]
    fn literal_count_counts_factor_leaves() {
        let network = sample_network();

        assert_eq!(factor_num_literal(&network, NodeId(3)).unwrap(), 3);
    }

    #[test]
    fn use_count_matches_leaf_occurrences_by_fanin_index() {
        let network = sample_network();

        assert_eq!(factor_num_used(&network, NodeId(4), NodeId(3)).unwrap(), 2);
        assert_eq!(factor_num_used(&network, NodeId(4), NodeId(2)).unwrap(), 1);
    }

    #[test]
    fn node_value_uses_formula_and_adds_literals_for_primary_output_fanout() {
        let network = sample_network();

        assert_eq!(node_value(&network, NodeId(3)).unwrap(), 4);
    }

    #[test]
    fn internal_node_without_fanout_keeps_legacy_infinite_value() {
        let mut network = FactorValueNetwork::new();
        let node = NodeId(1);
        network.add_node(
            FactorValueNode::new(node.0, "dead", NodeKind::Internal)
                .with_factor(FactorTree::and([leaf(0), leaf(1)])),
        );

        assert_eq!(node_value(&network, node).unwrap(), INFINITY_VALUE);
    }

    #[test]
    fn missing_fanin_reports_generic_error() {
        let network = sample_network();

        assert!(matches!(
            factor_num_used(&network, NodeId(4), NodeId(1)),
            Err(FactorValueError::MissingFanin { .. })
        ));
    }

    #[test]
    fn value_line_prints_infinite_and_finite_values_like_sis() {
        let network = sample_network();

        assert_eq!(value_line(&network, NodeId(1)).unwrap(), "a:\t(inf)\n");
        assert_eq!(value_line(&network, NodeId(3)).unwrap(), "f:\t4\n");
    }

    #[test]
    fn value_comparators_order_nodes_by_computed_value() {
        let network = sample_network();
        let mut nodes = [NodeId(1), NodeId(3), NodeId(4)];

        sort_by_value_increasing(&network, &mut nodes).unwrap();
        assert_eq!(nodes, [NodeId(4), NodeId(3), NodeId(1)]);

        sort_by_value_decreasing(&network, &mut nodes).unwrap();
        assert_eq!(nodes, [NodeId(1), NodeId(3), NodeId(4)]);
    }

    #[test]
    fn no_legacy_c_abi_or_dependency_metadata_tokens_are_present() {
        let source = include_str!("ft_value.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("be", "ad", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday", "1", "-", "8", "j", "8")));
    }
}
