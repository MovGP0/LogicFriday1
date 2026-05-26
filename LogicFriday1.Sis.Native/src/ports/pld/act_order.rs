//! Native Rust ordering model for `LogicSynthesis/sis/pld/act_order.c`.
//!
//! The C implementation builds a temporary OR root, optionally replaces nodes
//! with factored roots, computes deepest fanin levels, and emits fanins in
//! decreasing maximum-level order. This port keeps those mechanics on an owned
//! DAG. Callers that already have a factored representation can attach it with
//! `set_factor_root`; otherwise the orderer walks the graph as provided.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    PrimaryInput,
    PrimaryOutput,
    Zero,
    One,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActOrderNode {
    pub function: NodeFunction,
    fanins: Vec<NodeId>,
    factor_root: Option<NodeId>,
}

impl ActOrderNode {
    pub fn new(function: NodeFunction, fanins: impl Into<Vec<NodeId>>) -> Self {
        Self {
            function,
            fanins: fanins.into(),
            factor_root: None,
        }
    }

    pub fn primary_input() -> Self {
        Self::new(NodeFunction::PrimaryInput, Vec::new())
    }

    pub fn primary_output(fanin: NodeId) -> Self {
        Self::new(NodeFunction::PrimaryOutput, vec![fanin])
    }

    pub fn constant_zero() -> Self {
        Self::new(NodeFunction::Zero, Vec::new())
    }

    pub fn constant_one() -> Self {
        Self::new(NodeFunction::One, Vec::new())
    }

    pub fn internal(fanins: impl Into<Vec<NodeId>>) -> Self {
        Self::new(NodeFunction::Internal, fanins)
    }

    pub fn fanins(&self) -> &[NodeId] {
        &self.fanins
    }

    pub fn factor_root(&self) -> Option<NodeId> {
        self.factor_root
    }

    pub fn with_factor_root(mut self, root: NodeId) -> Self {
        self.factor_root = Some(root);
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActOrderGraph {
    nodes: Vec<ActOrderNode>,
}

impl ActOrderGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: ActOrderNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> ActOrderResult<&ActOrderNode> {
        self.nodes.get(id.0).ok_or(ActOrderError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> ActOrderResult<&mut ActOrderNode> {
        self.nodes
            .get_mut(id.0)
            .ok_or(ActOrderError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[ActOrderNode] {
        &self.nodes
    }

    pub fn set_factor_root(&mut self, node: NodeId, root: NodeId) -> ActOrderResult<()> {
        self.node(root)?;
        self.node_mut(node)?.factor_root = Some(root);
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActOrderError {
    UnknownNode(NodeId),
    EmptyPrimaryOutput(NodeId),
    CyclicGraph(NodeId),
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for ActOrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown act_order node {:?}", node),
            Self::EmptyPrimaryOutput(node) => {
                write!(f, "primary output {:?} has no driving fanin", node)
            }
            Self::CyclicGraph(node) => {
                write!(f, "act_order graph has a cycle reachable from {:?}", node)
            }
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native SIS graph integration")
            }
        }
    }
}

impl Error for ActOrderError {}

pub type ActOrderResult<T> = Result<T, ActOrderError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShuffleStep {
    pub value: u32,
}

pub fn alap_order_nodes(graph: &ActOrderGraph, nodes: &[NodeId]) -> ActOrderResult<Vec<NodeId>> {
    let mut root_graph = graph.clone();
    let mut root_fanins = Vec::new();

    for node in nodes {
        let selected = normalize_alap_input(&root_graph, *node)?;
        if root_graph.node(selected)?.function != NodeFunction::PrimaryInput {
            root_fanins.push(selected);
        }
    }

    let root = root_graph.add_node(ActOrderNode::internal(root_fanins));
    let root_table = build_root_table(&root_graph, root)?;
    let level_table = build_level_table(&root_graph, root, &root_table)?;
    let mut records = Vec::new();

    for (node, level) in level_table {
        let node_data = root_graph.node(node)?;
        if node_data.fanins().is_empty()
            && !matches!(node_data.function, NodeFunction::Zero | NodeFunction::One)
        {
            records.push(FaninRecord {
                max_level: level,
                node,
            });
        }
    }

    records.sort_by_key(|record| (Reverse(record.max_level), record.node));
    Ok(records.into_iter().map(|record| record.node).collect())
}

pub fn order_nodes(
    graph: &ActOrderGraph,
    nodes: &[NodeId],
    pi_only: bool,
) -> ActOrderResult<Vec<NodeId>> {
    let mut root_graph = graph.clone();
    let root = root_graph.add_node(ActOrderNode::internal(nodes.to_vec()));
    let root_table = build_root_table(&root_graph, root)?;
    let root = root_table.get(&root).copied().unwrap_or(root);
    let level_table = build_level_table(&root_graph, root, &root_table)?;
    let max_table = build_max_table(&root_graph, root, &level_table, &root_table)?;
    let mut order = Vec::new();
    let mut seen = BTreeSet::new();

    rec_order(
        &root_graph,
        root,
        &max_table,
        &root_table,
        &mut seen,
        &mut order,
        pi_only,
    )?;

    Ok(order)
}

pub fn shuffle_with_values(list: &[NodeId], values: &[u32]) -> Vec<NodeId> {
    let mut marked = vec![false; list.len()];
    let mut shuffled = Vec::with_capacity(list.len());

    for (i, value) in values.iter().copied().cycle().take(list.len()).enumerate() {
        let remaining = list.len() - i;
        let next_index = ((value % 32768) as usize * remaining) / 32768;
        let mut count = 0usize;

        for (index, is_marked) in marked.iter_mut().enumerate() {
            if *is_marked {
                continue;
            }
            if count == next_index {
                *is_marked = true;
                shuffled.push(list[index]);
                break;
            }
            count += 1;
        }
    }

    shuffled
}

pub fn shuffle_with_lcg_seed(list: &[NodeId], seed: u32) -> Vec<NodeId> {
    let mut state = seed;
    let values: Vec<_> = (0..list.len())
        .map(|_| {
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
            state & 0x7fff
        })
        .collect();
    shuffle_with_values(list, &values)
}

pub fn order_nodes_blocked<LegacyNodeVec>(
    _node_vec: &LegacyNodeVec,
    _pi_only: bool,
) -> ActOrderResult<Vec<NodeId>> {
    Err(missing_native_ports(
        "act_order SIS node_vec conversion and factoring",
    ))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FaninRecord {
    max_level: usize,
    node: NodeId,
}

fn normalize_alap_input(graph: &ActOrderGraph, node: NodeId) -> ActOrderResult<NodeId> {
    let node_data = graph.node(node)?;
    if node_data.function == NodeFunction::PrimaryOutput {
        node_data
            .fanins()
            .first()
            .copied()
            .ok_or(ActOrderError::EmptyPrimaryOutput(node))
    } else {
        Ok(node)
    }
}

fn build_root_table(
    graph: &ActOrderGraph,
    root: NodeId,
) -> ActOrderResult<BTreeMap<NodeId, NodeId>> {
    let mut table = BTreeMap::new();
    let mut visiting = HashSet::new();
    root_add(graph, root, &mut table, &mut visiting)?;
    Ok(table)
}

fn root_add(
    graph: &ActOrderGraph,
    node: NodeId,
    table: &mut BTreeMap<NodeId, NodeId>,
    visiting: &mut HashSet<NodeId>,
) -> ActOrderResult<()> {
    let node_data = graph.node(node)?;
    if node_data.fanins().is_empty() {
        return Ok(());
    }
    if table.contains_key(&node) {
        return Ok(());
    }
    if !visiting.insert(node) {
        return Err(ActOrderError::CyclicGraph(node));
    }

    if let Some(root) = node_data.factor_root() {
        graph.node(root)?;
        if root != node {
            table.insert(node, root);
        }
    }

    for fanin in node_data.fanins() {
        root_add(graph, *fanin, table, visiting)?;
    }

    visiting.remove(&node);
    Ok(())
}

fn build_level_table(
    graph: &ActOrderGraph,
    root: NodeId,
    root_table: &BTreeMap<NodeId, NodeId>,
) -> ActOrderResult<BTreeMap<NodeId, usize>> {
    let mut table = BTreeMap::from([(root, 0)]);
    let mut visiting = HashSet::new();
    level_add(graph, root, &mut table, root_table, &mut visiting)?;
    Ok(table)
}

fn level_add(
    graph: &ActOrderGraph,
    node: NodeId,
    level_table: &mut BTreeMap<NodeId, usize>,
    root_table: &BTreeMap<NodeId, NodeId>,
    visiting: &mut HashSet<NodeId>,
) -> ActOrderResult<()> {
    if !visiting.insert(node) {
        return Err(ActOrderError::CyclicGraph(node));
    }

    let current_level = *level_table.get(&node).unwrap_or(&0);
    let fanins: Vec<_> = graph
        .node(node)?
        .fanins()
        .iter()
        .map(|fanin| root_table.get(fanin).copied().unwrap_or(*fanin))
        .collect();

    for fanin in &fanins {
        let next_level = current_level + 1;
        match level_table.get_mut(fanin) {
            Some(old_level) if *old_level <= current_level => {
                *old_level = next_level;
            }
            None => {
                level_table.insert(*fanin, next_level);
            }
            _ => {}
        }
    }

    for fanin in fanins {
        level_add(graph, fanin, level_table, root_table, visiting)?;
    }

    visiting.remove(&node);
    Ok(())
}

fn build_max_table(
    graph: &ActOrderGraph,
    root: NodeId,
    level_table: &BTreeMap<NodeId, usize>,
    root_table: &BTreeMap<NodeId, NodeId>,
) -> ActOrderResult<BTreeMap<NodeId, usize>> {
    let mut table = BTreeMap::new();
    let mut visiting = HashSet::new();
    max_add(
        graph,
        root,
        &mut table,
        level_table,
        root_table,
        &mut visiting,
    )?;
    Ok(table)
}

fn max_add(
    graph: &ActOrderGraph,
    node: NodeId,
    max_table: &mut BTreeMap<NodeId, usize>,
    level_table: &BTreeMap<NodeId, usize>,
    root_table: &BTreeMap<NodeId, NodeId>,
    visiting: &mut HashSet<NodeId>,
) -> ActOrderResult<usize> {
    if let Some(max) = max_table.get(&node) {
        return Ok(*max);
    }
    if !visiting.insert(node) {
        return Err(ActOrderError::CyclicGraph(node));
    }

    let mut max = *level_table.get(&node).unwrap_or(&0);
    let fanins: Vec<_> = graph
        .node(node)?
        .fanins()
        .iter()
        .map(|fanin| root_table.get(fanin).copied().unwrap_or(*fanin))
        .collect();

    for fanin in fanins {
        max = max.max(max_add(
            graph,
            fanin,
            max_table,
            level_table,
            root_table,
            visiting,
        )?);
    }

    visiting.remove(&node);
    max_table.insert(node, max);
    Ok(max)
}

fn rec_order(
    graph: &ActOrderGraph,
    node: NodeId,
    max_table: &BTreeMap<NodeId, usize>,
    root_table: &BTreeMap<NodeId, NodeId>,
    seen: &mut BTreeSet<NodeId>,
    order: &mut Vec<NodeId>,
    pi_only: bool,
) -> ActOrderResult<()> {
    let node_data = graph.node(node)?;
    if node_data.fanins().is_empty() {
        if !matches!(node_data.function, NodeFunction::Zero | NodeFunction::One) {
            order.push(node);
        }
        return Ok(());
    }

    let mut iv_table = BTreeMap::new();
    let mut records = Vec::new();

    for fanin in node_data.fanins() {
        let mut ordered_fanin = *fanin;
        if !pi_only {
            if let Some(root) = root_table.get(fanin) {
                iv_table.insert(*root, *fanin);
                ordered_fanin = *root;
            }
        }
        records.push(FaninRecord {
            max_level: *max_table.get(&ordered_fanin).unwrap_or(&0),
            node: ordered_fanin,
        });
    }

    records.sort_by_key(|record| (Reverse(record.max_level), record.node));

    for record in records {
        if seen.insert(record.node) {
            rec_order(
                graph,
                record.node,
                max_table,
                root_table,
                seen,
                order,
                pi_only,
            )?;
            if !pi_only {
                if let Some(intermediate) = iv_table.get(&record.node) {
                    order.push(*intermediate);
                }
            }
        }
    }

    Ok(())
}

fn missing_native_ports(operation: &'static str) -> ActOrderError {
    ActOrderError::MissingNativePorts { operation }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> (ActOrderGraph, NodeId, NodeId, NodeId, NodeId, NodeId) {
        let mut graph = ActOrderGraph::new();
        let a = graph.add_node(ActOrderNode::primary_input());
        let b = graph.add_node(ActOrderNode::primary_input());
        let c = graph.add_node(ActOrderNode::primary_input());
        let n1 = graph.add_node(ActOrderNode::internal(vec![a, b]));
        let n2 = graph.add_node(ActOrderNode::internal(vec![n1, c]));
        (graph, a, b, c, n1, n2)
    }

    #[test]
    fn order_nodes_emits_deepest_fanins_first() {
        let (graph, a, b, c, _n1, n2) = sample_graph();

        let order = order_nodes(&graph, &[n2], true).unwrap();

        assert_eq!(order, vec![a, b, c]);
    }

    #[test]
    fn alap_order_ignores_primary_inputs_and_unwraps_primary_outputs() {
        let (mut graph, a, b, c, _n1, n2) = sample_graph();
        let po = graph.add_node(ActOrderNode::primary_output(n2));

        let order = alap_order_nodes(&graph, &[a, po]).unwrap();

        assert_eq!(order, vec![a, b, c]);
    }

    #[test]
    fn order_nodes_includes_original_node_after_factored_root_when_not_pi_only() {
        let mut graph = ActOrderGraph::new();
        let a = graph.add_node(ActOrderNode::primary_input());
        let b = graph.add_node(ActOrderNode::primary_input());
        let factored = graph.add_node(ActOrderNode::internal(vec![a, b]));
        let original =
            graph.add_node(ActOrderNode::internal(vec![a, b]).with_factor_root(factored));

        let order = order_nodes(&graph, &[original], false).unwrap();

        assert_eq!(order, vec![a, b, original]);
    }

    #[test]
    fn order_nodes_suppresses_factored_original_node_when_pi_only() {
        let mut graph = ActOrderGraph::new();
        let a = graph.add_node(ActOrderNode::primary_input());
        let b = graph.add_node(ActOrderNode::primary_input());
        let factored = graph.add_node(ActOrderNode::internal(vec![a, b]));
        let original =
            graph.add_node(ActOrderNode::internal(vec![a, b]).with_factor_root(factored));

        let order = order_nodes(&graph, &[original], true).unwrap();

        assert_eq!(order, vec![a, b]);
    }

    #[test]
    fn constants_are_not_returned_as_primary_inputs() {
        let mut graph = ActOrderGraph::new();
        let zero = graph.add_node(ActOrderNode::constant_zero());
        let one = graph.add_node(ActOrderNode::constant_one());
        let n = graph.add_node(ActOrderNode::internal(vec![zero, one]));

        let order = order_nodes(&graph, &[n], true).unwrap();

        assert_eq!(order, Vec::<NodeId>::new());
    }

    #[test]
    fn shuffle_with_values_matches_c_index_selection() {
        let list = vec![NodeId(0), NodeId(1), NodeId(2), NodeId(3)];

        let shuffled = shuffle_with_values(&list, &[0, 32767, 16384, 0]);

        assert_eq!(shuffled, vec![NodeId(0), NodeId(3), NodeId(2), NodeId(1)]);
    }

    #[test]
    fn invalid_legacy_entry_reports_generic_runtime_diagnostic() {
        let result = order_nodes_blocked(&(), true);

        assert_eq!(
            result,
            Err(ActOrderError::MissingNativePorts {
                operation: "act_order SIS node_vec conversion and factoring"
            })
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_bead_metadata_are_present_in_this_port() {
        let source = include_str!("act_order.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday", "1-")));
    }
}
