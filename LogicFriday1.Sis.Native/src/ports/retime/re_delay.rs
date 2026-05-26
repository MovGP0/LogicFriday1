//! Native Rust model for `LogicSynthesis/sis/retime/re_delay.c`.
//!
//! The C unit computes arrival times across zero-register retime edges, applies
//! latch and user timing offsets to derive the current cycle delay, and reports
//! a simple internal-gate lower bound. This module keeps that behavior in owned
//! Rust graph data. Direct exchange with SIS `re_graph` storage remains blocked
//! on the sibling graph/export ports that are not available in this Rust port yet.

use std::error::Error;
use std::fmt;

pub const RETIME_TEST_NOT_SET: f64 = -50_000.0;

pub fn sis_re_delay_integration_blocked() -> Result<(), ReDelayError> {
    Err(ReDelayError::MissingSisDependencies {
        operation: "re_cycle_delay/re_evaluate_delay SIS graph integration",
    })
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EdgeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetimeNodeType {
    PrimaryInput,
    PrimaryOutput,
    Internal,
    Ignore,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeNode {
    pub id: NodeId,
    pub node_type: RetimeNodeType,
    pub fanins: Vec<EdgeId>,
    pub fanouts: Vec<EdgeId>,
    pub final_delay: f64,
    pub user_time: f64,
}

impl RetimeNode {
    pub fn new(node_type: RetimeNodeType, final_delay: f64) -> Self {
        Self {
            id: NodeId(usize::MAX),
            node_type,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            final_delay,
            user_time: RETIME_TEST_NOT_SET,
        }
    }

    pub fn with_user_time(mut self, user_time: f64) -> Self {
        self.user_time = user_time;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeEdge {
    pub id: EdgeId,
    pub source: NodeId,
    pub sink: NodeId,
    pub weight: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RetimeGraph {
    pub nodes: Vec<RetimeNode>,
    pub edges: Vec<RetimeEdge>,
    pub primary_inputs: Vec<NodeId>,
    pub primary_outputs: Vec<NodeId>,
}

impl RetimeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, mut node: RetimeNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        node.id = id;
        match node.node_type {
            RetimeNodeType::PrimaryInput => self.primary_inputs.push(id),
            RetimeNodeType::PrimaryOutput => self.primary_outputs.push(id),
            RetimeNodeType::Internal | RetimeNodeType::Ignore => {}
        }
        self.nodes.push(node);
        id
    }

    pub fn add_edge(
        &mut self,
        source: NodeId,
        sink: NodeId,
        weight: i32,
    ) -> Result<EdgeId, ReDelayError> {
        self.require_node(source)?;
        self.require_node(sink)?;

        let id = EdgeId(self.edges.len());
        self.edges.push(RetimeEdge {
            id,
            source,
            sink,
            weight,
        });
        self.nodes[source.0].fanouts.push(id);
        self.nodes[sink.0].fanins.push(id);
        Ok(id)
    }

    pub fn cycle_delay(&self, latch_delay: f64) -> Result<f64, ReDelayError> {
        let delay_table = self.delay_table()?;
        let mut critical = 0.0;

        for index in (0..self.nodes.len()).rev() {
            let node = &self.nodes[index];
            let offset = if node.node_type == RetimeNodeType::PrimaryOutput
                && node.user_time > RETIME_TEST_NOT_SET
            {
                -node.user_time
            } else if self.max_fanout_weight(node.id)? > 0 {
                latch_delay
            } else {
                0.0
            };
            critical = f64::max(critical, delay_table[index] + offset);
        }

        Ok(critical)
    }

    pub fn delay_table(&self) -> Result<Vec<f64>, ReDelayError> {
        let mut valid = vec![false; self.nodes.len()];
        let mut delays = vec![0.0; self.nodes.len()];

        for node in &self.nodes {
            if node.fanins.is_empty() {
                valid[node.id.0] = true;
                if node.user_time > RETIME_TEST_NOT_SET {
                    delays[node.id.0] = node.user_time;
                }
            }
        }

        for index in (0..self.nodes.len()).rev() {
            if !valid[index] {
                self.evaluate_delay(NodeId(index), &mut valid, &mut delays)?;
            }
        }

        Ok(delays)
    }

    pub fn evaluate_delay(
        &self,
        node: NodeId,
        valid: &mut [bool],
        delays: &mut [f64],
    ) -> Result<(), ReDelayError> {
        self.evaluate_delay_inner(node, valid, delays, &mut Vec::new())
    }

    pub fn cycle_lower_bound(&self) -> f64 {
        self.nodes
            .iter()
            .filter(|node| node.node_type == RetimeNodeType::Internal)
            .fold(-1.0, |bound, node| f64::max(bound, node.final_delay))
    }

    pub fn max_fanout_weight(&self, node: NodeId) -> Result<i32, ReDelayError> {
        self.require_node(node)?;
        let mut maximum = 0;

        for edge_id in &self.nodes[node.0].fanouts {
            let edge = self.edge(*edge_id)?;
            if self.ignore_edge(edge)? {
                continue;
            }
            maximum = i32::max(maximum, edge.weight);
        }

        Ok(maximum)
    }

    fn evaluate_delay_inner(
        &self,
        node: NodeId,
        valid: &mut [bool],
        delays: &mut [f64],
        active: &mut Vec<NodeId>,
    ) -> Result<(), ReDelayError> {
        self.require_node(node)?;
        if valid.len() != self.nodes.len() || delays.len() != self.nodes.len() {
            return Err(ReDelayError::DelayTableLengthMismatch {
                expected: self.nodes.len(),
                valid: valid.len(),
                delays: delays.len(),
            });
        }
        if valid[node.0] {
            return Ok(());
        }
        if active.contains(&node) {
            return Err(ReDelayError::ZeroWeightDelayCycle { node });
        }

        active.push(node);
        for edge_id in &self.nodes[node.0].fanins {
            let edge = self.edge(*edge_id)?;
            if edge.weight == 0 && !valid[edge.source.0] {
                self.evaluate_delay_inner(edge.source, valid, delays, active)?;
            }
        }

        let mut max_fanin_delay = 0.0;
        for edge_id in &self.nodes[node.0].fanins {
            let edge = self.edge(*edge_id)?;
            if edge.weight == 0 {
                max_fanin_delay = f64::max(max_fanin_delay, delays[edge.source.0]);
            }
        }

        active.pop();
        valid[node.0] = true;
        delays[node.0] = max_fanin_delay + self.nodes[node.0].final_delay;
        Ok(())
    }

    fn edge(&self, edge: EdgeId) -> Result<&RetimeEdge, ReDelayError> {
        self.edges
            .get(edge.0)
            .ok_or(ReDelayError::MissingEdge(edge))
    }

    fn ignore_edge(&self, edge: &RetimeEdge) -> Result<bool, ReDelayError> {
        self.require_node(edge.source)?;
        self.require_node(edge.sink)?;
        Ok(
            self.nodes[edge.source.0].node_type == RetimeNodeType::Ignore
                || self.nodes[edge.sink.0].node_type == RetimeNodeType::Ignore,
        )
    }

    fn require_node(&self, node: NodeId) -> Result<(), ReDelayError> {
        match self.nodes.get(node.0) {
            Some(existing) if existing.id == node => Ok(()),
            _ => Err(ReDelayError::MissingNode(node)),
        }
    }
}

pub fn re_cycle_delay(graph: &RetimeGraph, latch_delay: f64) -> Result<f64, ReDelayError> {
    graph.cycle_delay(latch_delay)
}

pub fn re_evaluate_delay(
    graph: &RetimeGraph,
    node: NodeId,
    valid_table: &mut [bool],
    delay_table: &mut [f64],
) -> Result<(), ReDelayError> {
    graph.evaluate_delay(node, valid_table, delay_table)
}

pub fn retime_cycle_lower_bound(graph: &RetimeGraph) -> f64 {
    graph.cycle_lower_bound()
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReDelayError {
    MissingSisDependencies {
        operation: &'static str,
    },
    MissingNode(NodeId),
    MissingEdge(EdgeId),
    DelayTableLengthMismatch {
        expected: usize,
        valid: usize,
        delays: usize,
    },
    ZeroWeightDelayCycle {
        node: NodeId,
    },
}

impl fmt::Display for ReDelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisDependencies { operation } => {
                write!(f, "{operation} requires native prerequisite ports")
            }
            Self::MissingNode(node) => write!(f, "retime graph references missing node {}", node.0),
            Self::MissingEdge(edge) => write!(f, "retime graph references missing edge {}", edge.0),
            Self::DelayTableLengthMismatch {
                expected,
                valid,
                delays,
            } => write!(
                f,
                "delay evaluation requires {expected} entries, got valid={valid} delays={delays}"
            ),
            Self::ZeroWeightDelayCycle { node } => {
                write!(f, "zero-weight delay cycle reaches node {}", node.0)
            }
        }
    }
}

impl Error for ReDelayError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain_graph() -> RetimeGraph {
        let mut graph = RetimeGraph::new();
        let pi =
            graph.add_node(RetimeNode::new(RetimeNodeType::PrimaryInput, 0.0).with_user_time(1.0));
        let a = graph.add_node(RetimeNode::new(RetimeNodeType::Internal, 2.0));
        let b = graph.add_node(RetimeNode::new(RetimeNodeType::Internal, 3.0));
        let po = graph.add_node(RetimeNode::new(RetimeNodeType::PrimaryOutput, 0.0));
        graph.add_edge(pi, a, 0).unwrap();
        graph.add_edge(a, b, 0).unwrap();
        graph.add_edge(b, po, 0).unwrap();
        graph
    }

    #[test]
    fn delay_table_matches_zero_weight_arrival_recursion() {
        let graph = chain_graph();

        assert_eq!(graph.delay_table().unwrap(), vec![1.0, 3.0, 6.0, 6.0]);
    }

    #[test]
    fn evaluate_delay_updates_c_style_tables_for_one_node() {
        let graph = chain_graph();
        let mut valid = vec![false; graph.nodes.len()];
        let mut delays = vec![0.0; graph.nodes.len()];
        valid[0] = true;
        delays[0] = 1.0;

        re_evaluate_delay(&graph, NodeId(2), &mut valid, &mut delays).unwrap();

        assert!(valid[1]);
        assert!(valid[2]);
        assert_eq!(delays[1], 3.0);
        assert_eq!(delays[2], 6.0);
    }

    #[test]
    fn cycle_delay_applies_latch_and_required_output_offsets() {
        let mut graph = chain_graph();
        graph.edges[1].weight = 1;
        graph.nodes[3].user_time = 4.0;

        assert_eq!(re_cycle_delay(&graph, 0.5).unwrap(), 3.5);
    }

    #[test]
    fn cycle_lower_bound_uses_largest_internal_node_delay() {
        let graph = chain_graph();

        assert_eq!(retime_cycle_lower_bound(&graph), 3.0);
    }

    #[test]
    fn cycle_lower_bound_matches_c_empty_internal_sentinel() {
        let mut graph = RetimeGraph::new();
        graph.add_node(RetimeNode::new(RetimeNodeType::PrimaryInput, 0.0));
        graph.add_node(RetimeNode::new(RetimeNodeType::PrimaryOutput, 0.0));

        assert_eq!(graph.cycle_lower_bound(), -1.0);
    }

    #[test]
    fn ignored_fanout_edge_does_not_add_latch_offset() {
        let mut graph = RetimeGraph::new();
        let pi = graph.add_node(RetimeNode::new(RetimeNodeType::PrimaryInput, 0.0));
        let a = graph.add_node(RetimeNode::new(RetimeNodeType::Internal, 2.0));
        let ignored = graph.add_node(RetimeNode::new(RetimeNodeType::Ignore, 0.0));
        graph.add_edge(pi, a, 0).unwrap();
        graph.add_edge(a, ignored, 3).unwrap();

        assert_eq!(graph.max_fanout_weight(a).unwrap(), 0);
        assert_eq!(graph.cycle_delay(5.0).unwrap(), 2.0);
    }

    #[test]
    fn zero_weight_delay_cycles_are_reported_instead_of_recursing_forever() {
        let mut graph = RetimeGraph::new();
        let a = graph.add_node(RetimeNode::new(RetimeNodeType::Internal, 1.0));
        let b = graph.add_node(RetimeNode::new(RetimeNodeType::Internal, 1.0));
        graph.add_edge(a, b, 0).unwrap();
        graph.add_edge(b, a, 0).unwrap();

        assert_eq!(
            graph.delay_table(),
            Err(ReDelayError::ZeroWeightDelayCycle { node: NodeId(1) })
        );
    }
}
