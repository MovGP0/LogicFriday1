//! Native Rust port of `LogicSynthesis/sis/retime/retime_util.c`.
//!
//! The C unit owns the small retiming graph substrate: graph allocation,
//! node/edge duplication, indexed accessors, and edge insertion with fanin and
//! fanout cross-links. This module models that behavior with owned Rust
//! indices instead of raw `re_node *`/`re_edge *` pointers. Direct conversion to
//! and from SIS `node_t`/`latch_t` objects remains blocked on the native node,
//! latch, and sibling retime ports listed in `REQUIRED_PORT_BEADS`.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub const RETIME_NOT_SET: i32 = -1;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EdgeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchSynchType {
    ActiveHigh,
    ActiveLow,
    RisingEdge,
    FallingEdge,
    Combinational,
    Asynch,
    Unknown,
}

impl Default for LatchSynchType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetimeNodeType {
    PrimaryInput,
    PrimaryOutput,
    Internal,
    Ignore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead: &'static str,
    pub c_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_BEADS: &[PortDependency] = &[
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.230",
        c_file: "LogicSynthesis/sis/latch/latch.c",
        reason: "native latch_t identity and latch metadata carried by retime edges",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.313",
        c_file: "LogicSynthesis/sis/node/fan.c",
        reason: "native fanin/fanout traversal when building retime graphs from SIS nodes",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.318",
        c_file: "LogicSynthesis/sis/node/node.c",
        reason: "native node_t identity stored on retime nodes",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.422",
        c_file: "LogicSynthesis/sis/retime/re_util.c",
        reason: "graph construction from SIS networks and retime graph traversal helpers",
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetimeUtilError {
    MissingSisPorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
    MissingNode(NodeId),
    MissingEdge(EdgeId),
    NegativeWeight(i32),
}

impl fmt::Display for RetimeUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisPorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires {} native SIS prerequisite ports",
                dependencies.len()
            ),
            Self::MissingNode(node) => write!(f, "retime graph references missing node {}", node.0),
            Self::MissingEdge(edge) => write!(f, "retime graph references missing edge {}", edge.0),
            Self::NegativeWeight(weight) => {
                write!(f, "retime edge weight must be non-negative, got {weight}")
            }
        }
    }
}

impl Error for RetimeUtilError {}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeNode<N = usize> {
    pub id: NodeId,
    pub kind: RetimeNodeType,
    pub lp_index: i32,
    pub node: Option<N>,
    pub fanins: Vec<EdgeId>,
    pub fanouts: Vec<EdgeId>,
    pub scaled_delay: i32,
    pub final_area: f64,
    pub final_delay: f64,
    pub user_time: f64,
    pub scaled_user_time: i32,
}

impl<N> RetimeNode<N> {
    pub fn new(id: NodeId, kind: RetimeNodeType, node: Option<N>) -> Self {
        Self {
            id,
            kind,
            lp_index: 0,
            node,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            scaled_delay: RETIME_NOT_SET,
            final_area: 0.0,
            final_delay: 0.0,
            user_time: 0.0,
            scaled_user_time: 0,
        }
    }

    pub fn is_ignored(&self) -> bool {
        self.kind == RetimeNodeType::Ignore
    }

    pub fn is_host_vertex(&self) -> bool {
        matches!(
            self.kind,
            RetimeNodeType::PrimaryInput | RetimeNodeType::PrimaryOutput
        )
    }

    pub fn duplicate_without_links(&self) -> Self
    where
        N: Clone,
    {
        Self {
            id: self.id,
            kind: self.kind,
            lp_index: self.lp_index,
            node: self.node.clone(),
            fanins: Vec::new(),
            fanouts: Vec::new(),
            scaled_delay: RETIME_NOT_SET,
            final_area: self.final_area,
            final_delay: self.final_delay,
            user_time: self.user_time,
            scaled_user_time: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeEdge<L = usize> {
    pub id: EdgeId,
    pub source: NodeId,
    pub sink: NodeId,
    pub sink_fanin_id: usize,
    pub weight: usize,
    pub breadth: f64,
    pub temp_breadth: f64,
    pub latches: Vec<L>,
    pub initial_values: Vec<i32>,
    pub num_val_alloc: usize,
}

impl<L> RetimeEdge<L> {
    pub fn new(
        id: EdgeId,
        source: NodeId,
        sink: NodeId,
        sink_fanin_id: usize,
        weight: usize,
        breadth: f64,
    ) -> Self {
        Self {
            id,
            source,
            sink,
            sink_fanin_id,
            weight,
            breadth,
            temp_breadth: breadth,
            latches: Vec::new(),
            initial_values: Vec::new(),
            num_val_alloc: 0,
        }
    }

    pub fn duplicate_without_endpoints(&self) -> Self
    where
        L: Clone,
    {
        Self {
            id: self.id,
            source: self.source,
            sink: self.sink,
            sink_fanin_id: self.sink_fanin_id,
            weight: self.weight,
            breadth: self.breadth,
            temp_breadth: self.breadth,
            latches: self.latches.iter().take(self.weight).cloned().collect(),
            initial_values: self
                .initial_values
                .iter()
                .take(self.weight)
                .copied()
                .collect(),
            num_val_alloc: self.num_val_alloc,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeGraph<N = usize, L = usize> {
    pub nodes: Vec<RetimeNode<N>>,
    pub edges: Vec<RetimeEdge<L>>,
    pub primary_inputs: Vec<NodeId>,
    pub primary_outputs: Vec<NodeId>,
    pub s_type: LatchSynchType,
    pub control_name: Option<String>,
}

impl<N, L> Default for RetimeGraph<N, L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N, L> RetimeGraph<N, L> {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            primary_inputs: Vec::new(),
            primary_outputs: Vec::new(),
            s_type: LatchSynchType::Unknown,
            control_name: None,
        }
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn num_edges(&self) -> usize {
        self.edges.len()
    }

    pub fn num_primary_inputs(&self) -> usize {
        self.primary_inputs.len()
    }

    pub fn num_primary_outputs(&self) -> usize {
        self.primary_outputs.len()
    }

    pub fn num_internals(&self) -> usize {
        self.nodes
            .len()
            .saturating_sub(self.primary_inputs.len() + self.primary_outputs.len())
    }

    pub fn get_node(&self, index: usize) -> Option<&RetimeNode<N>> {
        self.nodes.get(index)
    }

    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut RetimeNode<N>> {
        self.nodes.get_mut(id.0).filter(|node| node.id == id)
    }

    pub fn get_edge(&self, index: usize) -> Option<&RetimeEdge<L>> {
        self.edges.get(index)
    }

    pub fn get_primary_input(&self, index: usize) -> Option<&RetimeNode<N>> {
        self.primary_inputs
            .get(index)
            .and_then(|id| self.get_node(id.0))
    }

    pub fn get_primary_output(&self, index: usize) -> Option<&RetimeNode<N>> {
        self.primary_outputs
            .get(index)
            .and_then(|id| self.get_node(id.0))
    }

    pub fn get_fanin(&self, node: NodeId, index: usize) -> Option<&RetimeEdge<L>> {
        let edge = self.get_node(node.0)?.fanins.get(index)?;
        self.get_edge(edge.0)
    }

    pub fn get_fanout(&self, node: NodeId, index: usize) -> Option<&RetimeEdge<L>> {
        let edge = self.get_node(node.0)?.fanouts.get(index)?;
        self.get_edge(edge.0)
    }

    pub fn add_node(&mut self, kind: RetimeNodeType, node: Option<N>) -> NodeId {
        let id = NodeId(self.nodes.len());
        let retime_node = RetimeNode::new(id, kind, node);

        match kind {
            RetimeNodeType::PrimaryInput => self.primary_inputs.push(id),
            RetimeNodeType::PrimaryOutput => self.primary_outputs.push(id),
            RetimeNodeType::Internal | RetimeNodeType::Ignore => {}
        }

        self.nodes.push(retime_node);
        id
    }

    pub fn create_edge(
        &mut self,
        source: NodeId,
        sink: NodeId,
        sink_fanin_id: usize,
        weight: usize,
        breadth: f64,
    ) -> Result<EdgeId, RetimeUtilError> {
        self.require_node(source)?;
        self.require_node(sink)?;

        let id = EdgeId(self.edges.len());
        let edge = RetimeEdge::new(id, source, sink, sink_fanin_id, weight, breadth);
        self.edges.push(edge);
        self.nodes[source.0].fanouts.push(id);
        self.nodes[sink.0].fanins.push(id);
        Ok(id)
    }

    pub fn edge_is_ignored(&self, edge: &RetimeEdge<L>) -> bool {
        self.nodes
            .get(edge.source.0)
            .is_none_or(RetimeNode::is_ignored)
            || self
                .nodes
                .get(edge.sink.0)
                .is_none_or(RetimeNode::is_ignored)
    }

    pub fn duplicate_without_ignored(&self) -> Result<Self, RetimeUtilError>
    where
        N: Clone,
        L: Clone,
    {
        let mut new_graph = Self::new();
        let mut node_refs = HashMap::new();
        let mut edge_refs = HashMap::new();

        for node in &self.nodes {
            if node.is_ignored() {
                continue;
            }

            let mut new_node = node.duplicate_without_links();
            new_node.id = NodeId(new_graph.nodes.len());
            let new_id = new_node.id;

            match new_node.kind {
                RetimeNodeType::PrimaryInput => new_graph.primary_inputs.push(new_id),
                RetimeNodeType::PrimaryOutput => new_graph.primary_outputs.push(new_id),
                RetimeNodeType::Internal | RetimeNodeType::Ignore => {}
            }

            new_graph.nodes.push(new_node);
            node_refs.insert(node.id, new_id);
        }

        for edge in &self.edges {
            if self.edge_is_ignored(edge) {
                continue;
            }

            let mut new_edge = edge.duplicate_without_endpoints();
            new_edge.id = EdgeId(new_graph.edges.len());
            let new_id = new_edge.id;
            new_graph.edges.push(new_edge);
            edge_refs.insert(edge.id, new_id);
        }

        for node in &self.nodes {
            if node.is_ignored() {
                continue;
            }
            let new_node_id = *node_refs
                .get(&node.id)
                .ok_or(RetimeUtilError::MissingNode(node.id))?;

            let fanins = node
                .fanins
                .iter()
                .filter_map(|edge| edge_refs.get(edge).copied())
                .collect();
            let fanouts = node
                .fanouts
                .iter()
                .filter_map(|edge| edge_refs.get(edge).copied())
                .collect();

            new_graph.nodes[new_node_id.0].fanins = fanins;
            new_graph.nodes[new_node_id.0].fanouts = fanouts;
        }

        for edge in &self.edges {
            let Some(new_edge_id) = edge_refs.get(&edge.id).copied() else {
                continue;
            };
            let source = *node_refs
                .get(&edge.source)
                .ok_or(RetimeUtilError::MissingNode(edge.source))?;
            let sink = *node_refs
                .get(&edge.sink)
                .ok_or(RetimeUtilError::MissingNode(edge.sink))?;

            let new_edge = &mut new_graph.edges[new_edge_id.0];
            new_edge.source = source;
            new_edge.sink = sink;
        }

        new_graph.s_type = self.s_type;
        new_graph.control_name = self.control_name.clone();
        Ok(new_graph)
    }

    fn require_node(&self, id: NodeId) -> Result<(), RetimeUtilError> {
        match self.nodes.get(id.0) {
            Some(node) if node.id == id => Ok(()),
            _ => Err(RetimeUtilError::MissingNode(id)),
        }
    }
}

pub fn required_port_beads() -> &'static [PortDependency] {
    REQUIRED_PORT_BEADS
}

pub fn graph_from_sis_network<N, L>() -> Result<RetimeGraph<N, L>, RetimeUtilError> {
    Err(RetimeUtilError::MissingSisPorts {
        operation: "retime_network_to_graph/re_graph_add_node/re_graph_add_edge",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

pub fn attach_sis_latches_to_edge<L>(_edge: &mut RetimeEdge<L>) -> Result<(), RetimeUtilError> {
    Err(RetimeUtilError::MissingSisPorts {
        operation: "retime edge latch attachment",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

pub fn c_weight_to_usize(weight: i32) -> Result<usize, RetimeUtilError> {
    usize::try_from(weight).map_err(|_| RetimeUtilError::NegativeWeight(weight))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> RetimeGraph<&'static str, &'static str> {
        let mut graph = RetimeGraph::new();
        graph.control_name = Some("clk".to_owned());
        graph.s_type = LatchSynchType::RisingEdge;

        let pi = graph.add_node(RetimeNodeType::PrimaryInput, Some("a"));
        let internal = graph.add_node(RetimeNodeType::Internal, Some("n1"));
        let po = graph.add_node(RetimeNodeType::PrimaryOutput, Some("z"));
        let ignored = graph.add_node(RetimeNodeType::Ignore, Some("dead"));

        let e0 = graph.create_edge(pi, internal, 0, 2, 1.5).unwrap();
        graph.edges[e0.0].latches = vec!["l0", "l1", "extra"];
        graph.edges[e0.0].initial_values = vec![0, 1, 3];
        graph.edges[e0.0].num_val_alloc = 3;
        graph.create_edge(internal, po, 0, 0, 2.5).unwrap();
        graph.create_edge(ignored, po, 1, 1, 7.0).unwrap();

        graph.nodes[internal.0].final_area = 9.0;
        graph.nodes[internal.0].final_delay = 3.0;
        graph.nodes[internal.0].user_time = 4.0;
        graph.nodes[internal.0].scaled_delay = 123;
        graph.nodes[internal.0].scaled_user_time = 456;
        graph
    }

    #[test]
    fn graph_defaults_match_re_graph_alloc() {
        let graph: RetimeGraph = RetimeGraph::new();

        assert_eq!(graph.num_nodes(), 0);
        assert_eq!(graph.num_edges(), 0);
        assert_eq!(graph.num_primary_inputs(), 0);
        assert_eq!(graph.num_primary_outputs(), 0);
        assert_eq!(graph.s_type, LatchSynchType::Unknown);
        assert_eq!(graph.control_name, None);
    }

    #[test]
    fn add_node_tracks_primary_partitions_and_defaults() {
        let mut graph: RetimeGraph<&str, usize> = RetimeGraph::new();
        let pi = graph.add_node(RetimeNodeType::PrimaryInput, Some("in"));
        let po = graph.add_node(RetimeNodeType::PrimaryOutput, Some("out"));
        let internal = graph.add_node(RetimeNodeType::Internal, Some("n"));

        assert_eq!(pi, NodeId(0));
        assert_eq!(po, NodeId(1));
        assert_eq!(internal, NodeId(2));
        assert_eq!(graph.primary_inputs, vec![pi]);
        assert_eq!(graph.primary_outputs, vec![po]);
        assert_eq!(graph.num_internals(), 1);
        assert_eq!(graph.nodes[internal.0].scaled_delay, RETIME_NOT_SET);
        assert_eq!(graph.nodes[internal.0].scaled_user_time, 0);
    }

    #[test]
    fn create_edge_cross_links_source_sink_and_accessors() {
        let mut graph: RetimeGraph<&str, usize> = RetimeGraph::new();
        let source = graph.add_node(RetimeNodeType::PrimaryInput, Some("a"));
        let sink = graph.add_node(RetimeNodeType::PrimaryOutput, Some("z"));

        let edge = graph.create_edge(source, sink, 3, 4, 2.25).unwrap();

        assert_eq!(edge, EdgeId(0));
        assert_eq!(graph.nodes[source.0].fanouts, vec![edge]);
        assert_eq!(graph.nodes[sink.0].fanins, vec![edge]);
        assert_eq!(graph.get_fanout(source, 0).unwrap().sink, sink);
        assert_eq!(graph.get_fanin(sink, 0).unwrap().source, source);
        assert_eq!(graph.get_edge(99), None);
        assert_eq!(graph.get_primary_input(0).unwrap().id, source);
        assert_eq!(graph.get_primary_output(0).unwrap().id, sink);
    }

    #[test]
    fn create_edge_rejects_missing_nodes() {
        let mut graph: RetimeGraph = RetimeGraph::new();
        let source = graph.add_node(RetimeNodeType::Internal, Some(1));

        assert_eq!(
            graph.create_edge(source, NodeId(99), 0, 0, 0.0),
            Err(RetimeUtilError::MissingNode(NodeId(99)))
        );
    }

    #[test]
    fn duplicate_matches_c_copy_rules_and_skips_ignored_edges() {
        let graph = sample_graph();
        let duplicate = graph.duplicate_without_ignored().unwrap();

        assert_eq!(duplicate.s_type, LatchSynchType::RisingEdge);
        assert_eq!(duplicate.control_name.as_deref(), Some("clk"));
        assert_eq!(duplicate.num_nodes(), 3);
        assert_eq!(duplicate.num_edges(), 2);
        assert_eq!(duplicate.primary_inputs, vec![NodeId(0)]);
        assert_eq!(duplicate.primary_outputs, vec![NodeId(2)]);

        let copied_internal = &duplicate.nodes[1];
        assert_eq!(copied_internal.node, Some("n1"));
        assert_eq!(copied_internal.final_area, 9.0);
        assert_eq!(copied_internal.final_delay, 3.0);
        assert_eq!(copied_internal.user_time, 4.0);
        assert_eq!(copied_internal.scaled_delay, RETIME_NOT_SET);
        assert_eq!(copied_internal.scaled_user_time, 0);

        let copied_edge = &duplicate.edges[0];
        assert_eq!(copied_edge.id, EdgeId(0));
        assert_eq!(copied_edge.source, NodeId(0));
        assert_eq!(copied_edge.sink, NodeId(1));
        assert_eq!(copied_edge.temp_breadth, copied_edge.breadth);
        assert_eq!(copied_edge.latches, vec!["l0", "l1"]);
        assert_eq!(copied_edge.initial_values, vec![0, 1]);
        assert_eq!(copied_edge.num_val_alloc, 3);
        assert_eq!(duplicate.nodes[2].fanins, vec![EdgeId(1)]);
    }

    #[test]
    fn edge_dup_resets_temp_breadth_and_copies_weight_entries() {
        let mut edge = RetimeEdge::new(EdgeId(7), NodeId(1), NodeId(2), 0, 1, 4.0);
        edge.temp_breadth = 99.0;
        edge.latches = vec!["first", "not-copied"];
        edge.initial_values = vec![3, 2];
        edge.num_val_alloc = 2;

        let copy = edge.duplicate_without_endpoints();

        assert_eq!(copy.temp_breadth, 4.0);
        assert_eq!(copy.latches, vec!["first"]);
        assert_eq!(copy.initial_values, vec![3]);
        assert_eq!(copy.num_val_alloc, 2);
    }

    #[test]
    fn dependency_scaffolds_report_bead_ids_and_sources() {
        assert!(required_port_beads().iter().any(|dependency| {
            dependency.bead == "LogicFriday1-8j8.2.6.318"
                && dependency.c_file == "LogicSynthesis/sis/node/node.c"
        }));
        assert!(required_port_beads().iter().any(|dependency| {
            dependency.bead == "LogicFriday1-8j8.2.6.230"
                && dependency.c_file == "LogicSynthesis/sis/latch/latch.c"
        }));
        assert_eq!(
            graph_from_sis_network::<usize, usize>(),
            Err(RetimeUtilError::MissingSisPorts {
                operation: "retime_network_to_graph/re_graph_add_node/re_graph_add_edge",
                dependencies: REQUIRED_PORT_BEADS,
            })
        );
    }

    #[test]
    fn c_weight_conversion_rejects_negative_values() {
        assert_eq!(c_weight_to_usize(0), Ok(0));
        assert_eq!(c_weight_to_usize(3), Ok(3));
        assert_eq!(
            c_weight_to_usize(-1),
            Err(RetimeUtilError::NegativeWeight(-1))
        );
    }
}
