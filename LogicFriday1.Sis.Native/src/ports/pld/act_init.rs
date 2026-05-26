//! Owned-data lifecycle helpers for ACT package state.
//!
//! The original C implementation installs and clears ACT slots on SIS nodes,
//! frees package-level traversal lists, recursively drops global ACTs through
//! fanout cones, and releases shared ACT vertices once. This port models those
//! responsibilities with Rust ownership and explicit diagnostics instead of
//! raw node slots and manual frees.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

pub const NO_VALUE: i32 = 4;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ActVertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ActNodeId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActVertex {
    pub value: i32,
    pub low: Option<ActVertexId>,
    pub high: Option<ActVertexId>,
}

impl ActVertex {
    pub fn terminal(value: i32) -> Self {
        Self {
            value,
            low: None,
            high: None,
        }
    }

    pub fn decision(low: ActVertexId, high: ActVertexId) -> Self {
        Self {
            value: NO_VALUE,
            low: Some(low),
            high: Some(high),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActGraph {
    vertices: HashMap<ActVertexId, ActVertex>,
    next_vertex: usize,
}

impl ActGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_vertex(&mut self, vertex: ActVertex) -> ActVertexId {
        let id = ActVertexId(self.next_vertex);
        self.next_vertex += 1;
        self.vertices.insert(id, vertex);
        id
    }

    pub fn vertex(&self, id: ActVertexId) -> Result<&ActVertex, ActInitError> {
        self.vertices
            .get(&id)
            .ok_or(ActInitError::MissingActVertex(id))
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn destroy_from(&mut self, root: ActVertexId) -> Result<ActDestroyReport, ActInitError> {
        let mut reachable = HashSet::new();
        self.collect_reachable(root, &mut reachable)?;
        let freed_vertices = reachable.len();
        for id in reachable {
            self.vertices.remove(&id);
        }

        Ok(ActDestroyReport { freed_vertices })
    }

    fn collect_reachable(
        &self,
        id: ActVertexId,
        reachable: &mut HashSet<ActVertexId>,
    ) -> Result<(), ActInitError> {
        if !reachable.insert(id) {
            return Ok(());
        }

        let vertex = self.vertex(id)?;
        if vertex.value == NO_VALUE {
            let low = vertex.low.ok_or(ActInitError::MissingDecisionChild {
                vertex: id,
                child: "low",
            })?;
            let high = vertex.high.ok_or(ActInitError::MissingDecisionChild {
                vertex: id,
                child: "high",
            })?;

            self.collect_reachable(low, reachable)?;
            self.collect_reachable(high, reachable)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Act {
    pub graph: ActGraph,
    pub root: Option<ActVertexId>,
    pub node_list: Vec<ActNodeId>,
    pub node_name: Option<String>,
}

impl Act {
    pub fn new(graph: ActGraph, root: Option<ActVertexId>) -> Self {
        Self {
            graph,
            root,
            node_list: Vec::new(),
            node_name: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActLocality {
    Global,
    Local,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActEntry {
    pub act: Act,
    pub order_style: i32,
}

impl ActEntry {
    pub fn new(act: Act, order_style: i32) -> Self {
        Self { act, order_style }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActSlot {
    pub local_act: Option<ActEntry>,
    pub global_act: Option<ActEntry>,
}

impl ActSlot {
    pub fn is_empty(&self) -> bool {
        self.local_act.is_none() && self.global_act.is_none()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActNode {
    pub id: ActNodeId,
    pub fanouts: Vec<ActNodeId>,
    pub slot: Option<ActSlot>,
}

impl ActNode {
    pub fn new(id: ActNodeId) -> Self {
        Self {
            id,
            fanouts: Vec::new(),
            slot: None,
        }
    }

    pub fn with_fanouts(mut self, fanouts: impl IntoIterator<Item = ActNodeId>) -> Self {
        self.fanouts = fanouts.into_iter().collect();
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActNetwork {
    nodes: HashMap<ActNodeId, ActNode>,
}

impl ActNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: ActNode) {
        self.nodes.insert(node.id, node);
    }

    pub fn node(&self, id: ActNodeId) -> Result<&ActNode, ActInitError> {
        self.nodes.get(&id).ok_or(ActInitError::MissingNode(id))
    }

    pub fn node_mut(&mut self, id: ActNodeId) -> Result<&mut ActNode, ActInitError> {
        self.nodes.get_mut(&id).ok_or(ActInitError::MissingNode(id))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActPackageState {
    global_lists: Vec<Vec<ActVertexId>>,
}

impl ActPackageState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_global_list(&mut self, list: Vec<ActVertexId>) {
        self.global_lists.push(list);
    }

    pub fn global_list_count(&self) -> usize {
        self.global_lists.len()
    }

    pub fn end(&mut self) -> ActEndReport {
        let freed_lists = self.global_lists.len();
        let freed_entries = self.global_lists.iter().map(Vec::len).sum();
        self.global_lists.clear();

        ActEndReport {
            freed_lists,
            freed_entries,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ActEndReport {
    pub freed_lists: usize,
    pub freed_entries: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ActDestroyReport {
    pub freed_vertices: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ActFreeReport {
    pub local: Option<ActDestroyReport>,
    pub global: Option<ActDestroyReport>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TransitiveFreeReport {
    pub nodes_cleared: usize,
    pub vertices_freed: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActInitError {
    MissingNode(ActNodeId),
    MissingActSlot(ActNodeId),
    MissingActRoot,
    MissingActVertex(ActVertexId),
    MissingDecisionChild {
        vertex: ActVertexId,
        child: &'static str,
    },
}

impl fmt::Display for ActInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(f, "node {} is not present", node.0),
            Self::MissingActSlot(node) => write!(f, "node {} has no ACT slot", node.0),
            Self::MissingActRoot => write!(f, "ACT entry has no root vertex"),
            Self::MissingActVertex(vertex) => write!(f, "ACT vertex {} is not present", vertex.0),
            Self::MissingDecisionChild { vertex, child } => {
                write!(
                    f,
                    "ACT decision vertex {} is missing {child} child",
                    vertex.0
                )
            }
        }
    }
}

impl Error for ActInitError {}

pub fn allocate_act_slot(node: &mut ActNode) {
    node.slot = Some(ActSlot::default());
}

pub fn free_local_and_global_acts(node: &mut ActNode) -> Result<ActFreeReport, ActInitError> {
    let Some(slot) = node.slot.as_mut() else {
        return Ok(ActFreeReport::default());
    };

    let local = destroy_entry(slot.local_act.take(), ActLocality::Local)?;
    let global = destroy_entry(slot.global_act.take(), ActLocality::Global)?;
    Ok(ActFreeReport { local, global })
}

pub fn free_act_slot(node: &mut ActNode) -> Option<ActSlot> {
    node.slot.take()
}

pub fn duplicate_act_slot_without_acts(_old: &ActNode, new: &mut ActNode) {
    new.slot = Some(ActSlot::default());
}

pub fn free_global_act_transitively(
    network: &mut ActNetwork,
    root: ActNodeId,
) -> Result<TransitiveFreeReport, ActInitError> {
    let mut visited = HashSet::new();
    free_global_act_transitively_from(network, root, &mut visited)
}

pub fn free_shared_act_vertices(
    graph: &mut ActGraph,
    root: Option<ActVertexId>,
) -> Result<ActDestroyReport, ActInitError> {
    match root {
        Some(root) => graph.destroy_from(root),
        None => Ok(ActDestroyReport::default()),
    }
}

fn destroy_entry(
    entry: Option<ActEntry>,
    locality: ActLocality,
) -> Result<Option<ActDestroyReport>, ActInitError> {
    let Some(mut entry) = entry else {
        return Ok(None);
    };

    let report = match entry.act.root {
        Some(root) => entry.act.graph.destroy_from(root)?,
        None => ActDestroyReport::default(),
    };

    if locality == ActLocality::Local {
        entry.act.node_list.clear();
    }

    Ok(Some(report))
}

fn free_global_act_transitively_from(
    network: &mut ActNetwork,
    node_id: ActNodeId,
    visited: &mut HashSet<ActNodeId>,
) -> Result<TransitiveFreeReport, ActInitError> {
    if !visited.insert(node_id) {
        return Ok(TransitiveFreeReport::default());
    }

    let mut report = TransitiveFreeReport::default();
    let fanouts;
    {
        let node = network.node_mut(node_id)?;
        let Some(slot) = node.slot.as_mut() else {
            return Err(ActInitError::MissingActSlot(node_id));
        };

        fanouts = node.fanouts.clone();
        if let Some(entry) = slot.global_act.take() {
            let destroy_report = destroy_entry(Some(entry), ActLocality::Global)?
                .ok_or(ActInitError::MissingActRoot)?;
            report.nodes_cleared += 1;
            report.vertices_freed += destroy_report.freed_vertices;
        } else {
            return Ok(report);
        }
    }

    for fanout in fanouts {
        let fanout_report = free_global_act_transitively_from(network, fanout, visited)?;
        report.nodes_cleared += fanout_report.nodes_cleared;
        report.vertices_freed += fanout_report.vertices_freed;
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_with_shared_decision_children() -> (ActGraph, ActVertexId) {
        let mut graph = ActGraph::new();
        let zero = graph.add_vertex(ActVertex::terminal(0));
        let one = graph.add_vertex(ActVertex::terminal(1));
        let left = graph.add_vertex(ActVertex::decision(zero, one));
        let right = graph.add_vertex(ActVertex::decision(zero, one));
        let root = graph.add_vertex(ActVertex::decision(left, right));

        (graph, root)
    }

    fn entry_with_graph() -> ActEntry {
        let (graph, root) = graph_with_shared_decision_children();
        ActEntry::new(Act::new(graph, Some(root)), 0)
    }

    #[test]
    fn package_end_clears_all_global_lists() {
        let mut state = ActPackageState::new();
        state.add_global_list(vec![ActVertexId(1), ActVertexId(2)]);
        state.add_global_list(vec![ActVertexId(3)]);

        let report = state.end();

        assert_eq!(
            report,
            ActEndReport {
                freed_lists: 2,
                freed_entries: 3,
            }
        );
        assert_eq!(state.global_list_count(), 0);
    }

    #[test]
    fn allocate_and_free_slot_matches_node_lifecycle() {
        let mut node = ActNode::new(ActNodeId(7));
        allocate_act_slot(&mut node);

        assert_eq!(node.slot, Some(ActSlot::default()));

        let freed = free_act_slot(&mut node);

        assert_eq!(freed, Some(ActSlot::default()));
        assert_eq!(node.slot, None);
    }

    #[test]
    fn local_and_global_acts_are_destroyed_and_slot_remains() {
        let mut node = ActNode::new(ActNodeId(1));
        allocate_act_slot(&mut node);
        let slot = node.slot.as_mut().unwrap();
        slot.local_act = Some(entry_with_graph());
        slot.global_act = Some(entry_with_graph());

        let report = free_local_and_global_acts(&mut node).unwrap();

        assert_eq!(
            report,
            ActFreeReport {
                local: Some(ActDestroyReport { freed_vertices: 5 }),
                global: Some(ActDestroyReport { freed_vertices: 5 }),
            }
        );
        assert_eq!(node.slot, Some(ActSlot::default()));
    }

    #[test]
    fn duplicate_slot_does_not_copy_acts() {
        let mut old = ActNode::new(ActNodeId(1));
        allocate_act_slot(&mut old);
        old.slot.as_mut().unwrap().global_act = Some(entry_with_graph());

        let mut new = ActNode::new(ActNodeId(2));
        duplicate_act_slot_without_acts(&old, &mut new);

        assert_eq!(new.slot, Some(ActSlot::default()));
    }

    #[test]
    fn transitive_free_stops_when_node_has_no_global_act() {
        let mut network = ActNetwork::new();
        let mut root = ActNode::new(ActNodeId(1)).with_fanouts([ActNodeId(2)]);
        allocate_act_slot(&mut root);
        root.slot.as_mut().unwrap().global_act = Some(entry_with_graph());

        let mut fanout = ActNode::new(ActNodeId(2));
        allocate_act_slot(&mut fanout);
        network.add_node(root);
        network.add_node(fanout);

        let report = free_global_act_transitively(&mut network, ActNodeId(1)).unwrap();

        assert_eq!(
            report,
            TransitiveFreeReport {
                nodes_cleared: 1,
                vertices_freed: 5,
            }
        );
        assert!(
            network
                .node(ActNodeId(1))
                .unwrap()
                .slot
                .as_ref()
                .unwrap()
                .global_act
                .is_none()
        );
        assert!(
            network
                .node(ActNodeId(2))
                .unwrap()
                .slot
                .as_ref()
                .unwrap()
                .global_act
                .is_none()
        );
    }

    #[test]
    fn transitive_free_recurses_through_fanouts_with_global_acts_once() {
        let mut network = ActNetwork::new();
        let mut root = ActNode::new(ActNodeId(1)).with_fanouts([ActNodeId(2), ActNodeId(3)]);
        let mut left = ActNode::new(ActNodeId(2)).with_fanouts([ActNodeId(3)]);
        let mut right = ActNode::new(ActNodeId(3));

        for node in [&mut root, &mut left, &mut right] {
            allocate_act_slot(node);
            node.slot.as_mut().unwrap().global_act = Some(entry_with_graph());
        }

        network.add_node(root);
        network.add_node(left);
        network.add_node(right);

        let report = free_global_act_transitively(&mut network, ActNodeId(1)).unwrap();

        assert_eq!(
            report,
            TransitiveFreeReport {
                nodes_cleared: 3,
                vertices_freed: 15,
            }
        );
    }

    #[test]
    fn shared_act_vertex_free_counts_each_reachable_vertex_once() {
        let (mut graph, root) = graph_with_shared_decision_children();

        let report = free_shared_act_vertices(&mut graph, Some(root)).unwrap();

        assert_eq!(report, ActDestroyReport { freed_vertices: 5 });
        assert_eq!(graph.vertex_count(), 0);
    }

    #[test]
    fn malformed_decision_reports_missing_child() {
        let mut graph = ActGraph::new();
        let root = graph.add_vertex(ActVertex {
            value: NO_VALUE,
            low: None,
            high: None,
        });

        let error = free_shared_act_vertices(&mut graph, Some(root)).unwrap_err();

        assert_eq!(
            error,
            ActInitError::MissingDecisionChild {
                vertex: root,
                child: "low",
            }
        );
    }
}
