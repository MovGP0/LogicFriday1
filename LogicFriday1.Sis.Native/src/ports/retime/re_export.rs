//! Native Rust model for `LogicSynthesis/sis/retime/re_export.c`.
//!
//! The C file exports small retiming graph metrics over `re_graph`, `re_node`,
//! and `re_edge`: min/max incident register weights, total/effective register
//! weights, node area totals, and forward/backward retimability predicates.
//! This module ports those behaviors to owned Rust data structures. Direct use
//! of the legacy SIS `re_graph` storage remains an explicit dependency error
//!; no legacy C ABI entry points are exposed here.

use std::error::Error;
use std::fmt;

pub const POS_LARGE: i32 = 10_000;

pub fn legacy_re_export_blocked() -> Result<(), ReExportError> {
    Err(ReExportError::MissingSisDependencies {
        operation: "retime re_export metrics over legacy SIS re_graph",
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetimeNodeType {
    PrimaryInput,
    PrimaryOutput,
    Internal,
    Ignore,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EdgeId(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeNode {
    pub id: NodeId,
    pub node_type: RetimeNodeType,
    pub fanins: Vec<EdgeId>,
    pub fanouts: Vec<EdgeId>,
    pub final_area: f64,
}

impl RetimeNode {
    pub fn new(id: NodeId, node_type: RetimeNodeType, final_area: f64) -> Self {
        Self {
            id,
            node_type,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            final_area,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
}

impl RetimeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node_type: RetimeNodeType, final_area: f64) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(RetimeNode::new(id, node_type, final_area));
        id
    }

    pub fn add_edge(
        &mut self,
        source: NodeId,
        sink: NodeId,
        weight: i32,
    ) -> Result<EdgeId, ReExportError> {
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

    pub fn min_fanin_weight(&self, node: NodeId) -> Result<i32, ReExportError> {
        self.fold_incident_weights(node, IncidentDirection::Fanin, POS_LARGE, i32::min)
    }

    pub fn min_fanout_weight(&self, node: NodeId) -> Result<i32, ReExportError> {
        self.fold_incident_weights(node, IncidentDirection::Fanout, POS_LARGE, i32::min)
    }

    pub fn max_fanin_weight(&self, node: NodeId) -> Result<i32, ReExportError> {
        self.fold_incident_weights(node, IncidentDirection::Fanin, 0, i32::max)
    }

    pub fn max_fanout_weight(&self, node: NodeId) -> Result<i32, ReExportError> {
        self.fold_incident_weights(node, IncidentDirection::Fanout, 0, i32::max)
    }

    pub fn sum_of_edge_weight(&self) -> i32 {
        self.edges
            .iter()
            .filter(|edge| !self.ignore_edge(edge.id).unwrap_or(true))
            .map(|edge| edge.weight)
            .sum()
    }

    pub fn effective_sum_edge_weight(&self) -> Result<i32, ReExportError> {
        let mut sum = 0;
        for node in &self.nodes {
            if node.node_type == RetimeNodeType::Ignore {
                continue;
            }
            sum += self.max_fanout_weight(node.id)?;
        }
        Ok(sum)
    }

    pub fn sum_node_area(&self) -> f64 {
        self.nodes.iter().map(|node| node.final_area).sum()
    }

    pub fn total_area(&self, register_area: f64) -> Result<f64, ReExportError> {
        Ok(self.sum_node_area() + register_area * f64::from(self.effective_sum_edge_weight()?))
    }

    pub fn node_retimable(&self, node: NodeId) -> Result<bool, ReExportError> {
        self.require_internal_node(node)?;
        Ok(self.min_fanin_weight(node)? != 0 || self.min_fanout_weight(node)? != 0)
    }

    pub fn node_forward_retimable(&self, node: NodeId) -> Result<bool, ReExportError> {
        self.require_internal_node(node)?;
        Ok(self.min_fanin_weight(node)? != 0)
    }

    pub fn node_backward_retimable(&self, node: NodeId) -> Result<bool, ReExportError> {
        self.require_internal_node(node)?;
        Ok(self.min_fanout_weight(node)? != 0)
    }

    pub fn ignore_edge(&self, edge: EdgeId) -> Result<bool, ReExportError> {
        let edge = self.edge(edge)?;
        Ok(self.node(edge.source)?.node_type == RetimeNodeType::Ignore
            || self.node(edge.sink)?.node_type == RetimeNodeType::Ignore)
    }

    fn fold_incident_weights(
        &self,
        node: NodeId,
        direction: IncidentDirection,
        initial: i32,
        fold: fn(i32, i32) -> i32,
    ) -> Result<i32, ReExportError> {
        let node_ref = self.node(node)?;
        let edge_ids = match direction {
            IncidentDirection::Fanin => &node_ref.fanins,
            IncidentDirection::Fanout => &node_ref.fanouts,
        };

        let mut value = initial;
        for edge_id in edge_ids {
            let edge = self.edge(*edge_id)?;
            if !self.ignore_edge(*edge_id)? {
                value = fold(value, edge.weight);
            }
        }
        Ok(value)
    }

    fn node(&self, node: NodeId) -> Result<&RetimeNode, ReExportError> {
        self.nodes
            .get(node.0)
            .ok_or(ReExportError::MissingNode(node))
    }

    fn edge(&self, edge: EdgeId) -> Result<&RetimeEdge, ReExportError> {
        self.edges
            .get(edge.0)
            .ok_or(ReExportError::MissingEdge(edge))
    }

    fn require_node(&self, node: NodeId) -> Result<(), ReExportError> {
        self.node(node).map(|_| ())
    }

    fn require_internal_node(&self, node: NodeId) -> Result<(), ReExportError> {
        let node_ref = self.node(node)?;
        if node_ref.node_type == RetimeNodeType::Internal {
            Ok(())
        } else {
            Err(ReExportError::CannotRetimeNonInternal {
                node,
                node_type: node_ref.node_type,
            })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IncidentDirection {
    Fanin,
    Fanout,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReExportError {
    MissingSisDependencies {
        operation: &'static str,
    },
    MissingNode(NodeId),
    MissingEdge(EdgeId),
    CannotRetimeNonInternal {
        node: NodeId,
        node_type: RetimeNodeType,
    },
}

impl fmt::Display for ReExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisDependencies { operation } => {
                write!(f, "{operation} requires native prerequisite ports")
            }
            Self::MissingNode(node) => write!(f, "missing retime node {}", node.0),
            Self::MissingEdge(edge) => write!(f, "missing retime edge {}", edge.0),
            Self::CannotRetimeNonInternal { node, node_type } => {
                write!(
                    f,
                    "cannot retime non-internal node {} ({node_type:?})",
                    node.0
                )
            }
        }
    }
}

impl Error for ReExportError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_with_ignored_edge() -> (RetimeGraph, NodeId, NodeId, NodeId, NodeId) {
        let mut graph = RetimeGraph::new();
        let input = graph.add_node(RetimeNodeType::PrimaryInput, 0.0);
        let left = graph.add_node(RetimeNodeType::Internal, 2.5);
        let right = graph.add_node(RetimeNodeType::Internal, 3.5);
        let ignored = graph.add_node(RetimeNodeType::Ignore, 11.0);

        graph.add_edge(input, left, 2).unwrap();
        graph.add_edge(left, right, 0).unwrap();
        graph.add_edge(right, left, 5).unwrap();
        graph.add_edge(left, ignored, 99).unwrap();
        graph.add_edge(ignored, right, 77).unwrap();

        (graph, input, left, right, ignored)
    }

    #[test]
    fn min_and_max_incident_weights_skip_ignored_edges() {
        let (graph, _, left, right, ignored) = graph_with_ignored_edge();

        assert_eq!(graph.min_fanin_weight(left), Ok(2));
        assert_eq!(graph.max_fanin_weight(left), Ok(5));
        assert_eq!(graph.min_fanout_weight(left), Ok(0));
        assert_eq!(graph.max_fanout_weight(left), Ok(0));
        assert_eq!(graph.min_fanin_weight(right), Ok(0));
        assert_eq!(graph.max_fanin_weight(right), Ok(0));
        assert_eq!(graph.min_fanin_weight(ignored), Ok(POS_LARGE));
        assert_eq!(graph.max_fanout_weight(ignored), Ok(0));
    }

    #[test]
    fn edge_sums_match_c_total_and_effective_register_counts() {
        let (graph, _, _, _, _) = graph_with_ignored_edge();

        assert_eq!(graph.sum_of_edge_weight(), 7);
        assert_eq!(graph.effective_sum_edge_weight(), Ok(7));
    }

    #[test]
    fn area_helpers_include_all_nodes_and_effective_register_area() {
        let (graph, _, _, _, _) = graph_with_ignored_edge();

        assert_eq!(graph.sum_node_area(), 17.0);
        assert_eq!(graph.total_area(10.0), Ok(87.0));
    }

    #[test]
    fn retimability_predicates_match_re_export_rules() {
        let mut graph = RetimeGraph::new();
        let input = graph.add_node(RetimeNodeType::PrimaryInput, 0.0);
        let a = graph.add_node(RetimeNodeType::Internal, 1.0);
        let b = graph.add_node(RetimeNodeType::Internal, 1.0);
        let c = graph.add_node(RetimeNodeType::Internal, 1.0);

        graph.add_edge(input, a, 0).unwrap();
        graph.add_edge(a, b, 0).unwrap();
        graph.add_edge(b, c, 3).unwrap();

        assert_eq!(graph.node_retimable(a), Ok(false));
        assert_eq!(graph.node_forward_retimable(a), Ok(false));
        assert_eq!(graph.node_backward_retimable(a), Ok(false));

        assert_eq!(graph.node_retimable(b), Ok(true));
        assert_eq!(graph.node_forward_retimable(b), Ok(false));
        assert_eq!(graph.node_backward_retimable(b), Ok(true));

        assert_eq!(graph.node_retimable(c), Ok(true));
        assert_eq!(graph.node_forward_retimable(c), Ok(true));
        assert_eq!(graph.node_backward_retimable(c), Ok(true));

        assert!(matches!(
            graph.node_retimable(input),
            Err(ReExportError::CannotRetimeNonInternal { .. })
        ));
    }

    #[test]
    fn empty_incident_sets_keep_c_sentinel_defaults() {
        let mut graph = RetimeGraph::new();
        let node = graph.add_node(RetimeNodeType::Internal, 0.0);

        assert_eq!(graph.min_fanin_weight(node), Ok(POS_LARGE));
        assert_eq!(graph.min_fanout_weight(node), Ok(POS_LARGE));
        assert_eq!(graph.max_fanin_weight(node), Ok(0));
        assert_eq!(graph.max_fanout_weight(node), Ok(0));
        assert_eq!(graph.node_retimable(node), Ok(true));
    }
}
