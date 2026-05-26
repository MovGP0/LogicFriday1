//! Native Rust model for `LogicSynthesis/sis/pld/xln_move_d.c`.
//!
//! The C file drives a delay-oriented fanin moving pass: optionally run a unit
//! delay trace, visit critical internal nodes in DFS order, try each
//! non-critical fanin, retrace delay after every successful move, and stop once
//! the requested infeasibility improvement has been reached. The real SIS pass
//! delegates feasibility and graph rewrites to `xln_node_move_fanin`; this port
//! models the orchestration natively and keeps SIS-bound entry points blocked
//! behind explicit dependency errors.

use std::error::Error;
use std::fmt;

pub const MAX_DIFF: usize = usize::MAX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MoveNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub critical: bool,
}

impl MoveNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            critical: false,
        }
    }

    pub fn with_fanins(mut self, fanins: Vec<NodeId>) -> Self {
        self.fanins = fanins;
        self
    }

    pub fn with_critical(mut self, critical: bool) -> Self {
        self.critical = critical;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MoveGraph {
    nodes: Vec<MoveNode>,
    delay_trace_count: usize,
}

impl MoveGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            delay_trace_count: 0,
        }
    }

    pub fn add_node(&mut self, node: MoveNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> XlnMoveDResult<&MoveNode> {
        self.nodes.get(id.0).ok_or(XlnMoveDError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> XlnMoveDResult<&mut MoveNode> {
        self.nodes
            .get_mut(id.0)
            .ok_or(XlnMoveDError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[MoveNode] {
        &self.nodes
    }

    pub fn delay_trace_count(&self) -> usize {
        self.delay_trace_count
    }

    pub fn trace_unit_delay(&mut self) {
        self.delay_trace_count += 1;
    }

    pub fn dfs_order(&self) -> Vec<NodeId> {
        (0..self.nodes.len()).map(NodeId).collect()
    }

    pub fn remove_fanin(&mut self, node: NodeId, fanin: NodeId) -> XlnMoveDResult<bool> {
        let target = self.node_mut(node)?;
        let before = target.fanins.len();
        target.fanins.retain(|candidate| *candidate != fanin);
        Ok(target.fanins.len() != before)
    }
}

impl Default for MoveGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoveOptions {
    pub move_fanins: bool,
    pub support: usize,
    pub max_fanins: usize,
    pub bound_alphas: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveMode {
    Delay,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoveAttempt {
    pub node: NodeId,
    pub fanin: NodeId,
    pub support: usize,
    pub bound_alphas: usize,
    pub mode: MoveMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MoveNetworkReport {
    pub visited_critical_nodes: usize,
    pub improvements: usize,
    pub delay_traces: usize,
}

pub trait MoveFaninBackend {
    fn move_fanin(&mut self, graph: &mut MoveGraph, attempt: MoveAttempt) -> XlnMoveDResult<bool>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemovingMoveBackend {
    movable_fanins: Vec<(NodeId, NodeId)>,
}

impl RemovingMoveBackend {
    pub fn new(movable_fanins: Vec<(NodeId, NodeId)>) -> Self {
        Self { movable_fanins }
    }
}

impl MoveFaninBackend for RemovingMoveBackend {
    fn move_fanin(&mut self, graph: &mut MoveGraph, attempt: MoveAttempt) -> XlnMoveDResult<bool> {
        let Some(position) = self
            .movable_fanins
            .iter()
            .position(|candidate| *candidate == (attempt.node, attempt.fanin))
        else {
            return Ok(false);
        };

        self.movable_fanins.remove(position);
        graph.remove_fanin(attempt.node, attempt.fanin)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnMoveDError {
    UnknownNode(NodeId),
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for XlnMoveDError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown xln_move_d node {:?}", node),
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation} is blocked by unported SIS C-file dependencies"
            ),
        }
    }
}

impl Error for XlnMoveDError {}

pub type XlnMoveDResult<T> = Result<T, XlnMoveDError>;

pub fn network_move_fanins_for_delay<B>(
    graph: &mut MoveGraph,
    options: MoveOptions,
    init_delay_trace: bool,
    backend: &mut B,
) -> XlnMoveDResult<MoveNetworkReport>
where
    B: MoveFaninBackend,
{
    if !options.move_fanins {
        return Ok(MoveNetworkReport {
            visited_critical_nodes: 0,
            improvements: 0,
            delay_traces: 0,
        });
    }

    let traces_before = graph.delay_trace_count();
    if init_delay_trace {
        graph.trace_unit_delay();
    }

    let mut visited_critical_nodes = 0;
    let mut improvements = 0;
    for node in graph.dfs_order() {
        if !graph.node(node)?.critical {
            continue;
        }
        visited_critical_nodes += 1;
        improvements += node_move_fanins_for_delay(
            graph,
            node,
            options.support,
            options.max_fanins,
            MAX_DIFF,
            options.bound_alphas,
            backend,
        )?;
    }

    Ok(MoveNetworkReport {
        visited_critical_nodes,
        improvements,
        delay_traces: graph.delay_trace_count() - traces_before,
    })
}

pub fn node_move_fanins_for_delay<B>(
    graph: &mut MoveGraph,
    node: NodeId,
    support: usize,
    max_fanins: usize,
    diff: usize,
    bound_alphas: usize,
    backend: &mut B,
) -> XlnMoveDResult<usize>
where
    B: MoveFaninBackend,
{
    let node_ref = graph.node(node)?;
    if node_ref.kind != NodeKind::Internal {
        return Ok(0);
    }

    let num_fanin = node_ref.fanins.len();
    if num_fanin > max_fanins {
        return Ok(0);
    }

    let infeasibility = if diff == 0 {
        match num_fanin.checked_sub(support) {
            Some(infeasibility) if infeasibility > 0 => infeasibility,
            _ => return Ok(0),
        }
    } else {
        diff
    };

    let fanin_snapshot = node_ref.fanins.clone();
    let mut improvement = 0;
    for fanin in fanin_snapshot {
        if graph.node(fanin)?.critical {
            continue;
        }

        let moved = backend.move_fanin(
            graph,
            MoveAttempt {
                node,
                fanin,
                support,
                bound_alphas,
                mode: MoveMode::Delay,
            },
        )?;
        if moved {
            graph.trace_unit_delay();
            improvement += 1;
        }
        if improvement == infeasibility {
            break;
        }
    }

    Ok(improvement)
}

pub fn network_move_fanins_for_delay_blocked<Network>(
    _network: &mut Network,
    _options: MoveOptions,
    _init_delay_trace: bool,
) -> XlnMoveDResult<MoveNetworkReport> {
    Err(XlnMoveDError::MissingNativePorts {
        operation: "xln_network_move_fanins_for_delay SIS integration",
    })
}

pub fn node_move_fanins_for_delay_blocked<Node>(
    _node: &mut Node,
    _support: usize,
    _max_fanins: usize,
    _diff: usize,
    _bound_alphas: usize,
) -> XlnMoveDResult<usize> {
    Err(XlnMoveDError::MissingNativePorts {
        operation: "xln_node_move_fanins_for_delay SIS integration",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> (MoveGraph, NodeId, NodeId, NodeId, NodeId) {
        let mut graph = MoveGraph::new();
        let a = graph.add_node(MoveNode::new("a", NodeKind::PrimaryInput));
        let b = graph.add_node(MoveNode::new("b", NodeKind::PrimaryInput));
        let c = graph.add_node(MoveNode::new("c", NodeKind::PrimaryInput).with_critical(true));
        let n = graph.add_node(
            MoveNode::new("n", NodeKind::Internal)
                .with_fanins(vec![a, b, c])
                .with_critical(true),
        );
        (graph, a, b, c, n)
    }

    #[test]
    fn disabled_network_pass_returns_without_delay_trace() {
        let (mut graph, a, _, _, n) = sample_graph();
        let mut backend = RemovingMoveBackend::new(vec![(n, a)]);

        let report = network_move_fanins_for_delay(
            &mut graph,
            MoveOptions {
                move_fanins: false,
                support: 2,
                max_fanins: 8,
                bound_alphas: 1,
            },
            true,
            &mut backend,
        )
        .unwrap();

        assert_eq!(
            report,
            MoveNetworkReport {
                visited_critical_nodes: 0,
                improvements: 0,
                delay_traces: 0,
            }
        );
        assert_eq!(graph.delay_trace_count(), 0);
    }

    #[test]
    fn node_move_skips_non_internal_and_oversized_nodes() {
        let (mut graph, a, _, _, n) = sample_graph();
        let mut backend = RemovingMoveBackend::new(vec![(n, a)]);

        assert_eq!(
            node_move_fanins_for_delay(&mut graph, a, 2, 8, MAX_DIFF, 1, &mut backend).unwrap(),
            0
        );
        assert_eq!(
            node_move_fanins_for_delay(&mut graph, n, 2, 2, MAX_DIFF, 1, &mut backend).unwrap(),
            0
        );
        assert_eq!(graph.node(n).unwrap().fanins.len(), 3);
    }

    #[test]
    fn diff_zero_moves_only_until_node_becomes_feasible() {
        let (mut graph, a, b, c, n) = sample_graph();
        let mut backend = RemovingMoveBackend::new(vec![(n, a), (n, b), (n, c)]);

        let moved = node_move_fanins_for_delay(&mut graph, n, 2, 8, 0, 1, &mut backend).unwrap();

        assert_eq!(moved, 1);
        assert_eq!(graph.delay_trace_count(), 1);
        assert_eq!(graph.node(n).unwrap().fanins, vec![b, c]);
    }

    #[test]
    fn delay_pass_skips_critical_fanins_and_retraces_after_successes() {
        let (mut graph, a, b, c, n) = sample_graph();
        let mut backend = RemovingMoveBackend::new(vec![(n, a), (n, b), (n, c)]);

        let report = network_move_fanins_for_delay(
            &mut graph,
            MoveOptions {
                move_fanins: true,
                support: 2,
                max_fanins: 8,
                bound_alphas: 1,
            },
            true,
            &mut backend,
        )
        .unwrap();

        assert_eq!(
            report,
            MoveNetworkReport {
                visited_critical_nodes: 2,
                improvements: 2,
                delay_traces: 3,
            }
        );
        assert_eq!(graph.node(n).unwrap().fanins, vec![c]);
    }

    #[test]
    fn bounded_diff_stops_after_requested_improvements() {
        let (mut graph, a, b, c, n) = sample_graph();
        let mut backend = RemovingMoveBackend::new(vec![(n, a), (n, b), (n, c)]);

        let moved = node_move_fanins_for_delay(&mut graph, n, 1, 8, 1, 1, &mut backend).unwrap();

        assert_eq!(moved, 1);
        assert_eq!(graph.node(n).unwrap().fanins, vec![b, c]);
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_move_d.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
