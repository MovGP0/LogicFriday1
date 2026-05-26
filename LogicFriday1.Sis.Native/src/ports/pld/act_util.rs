//! Owned Rust model for ACT utility behavior.
//!
//! The original utility layer mixed small graph algorithms with direct SIS
//! pointer-table mutation. This module keeps the graph behavior native and
//! explicit; routines that still need broader SIS integration return a generic
//! dependency diagnostic.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const MARK_COMPLEMENT_VALUE: i32 = 0;

pub type ActUtilResult<T> = Result<T, ActUtilError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActUtilError {
    MissingNativePorts {
        operation: &'static str,
    },
    MissingNode(NodeId),
    MissingNetworkNode(String),
    MissingCost(NodeId),
    MissingVertex(ActVertexId),
    MissingChild {
        vertex: ActVertexId,
        edge: ActEdge,
    },
    InvalidVertexValue {
        vertex: ActVertexId,
        value: ActValue,
    },
    InvalidTerminalChild {
        vertex: ActVertexId,
        edge: ActEdge,
    },
    InvalidInternalChild {
        vertex: ActVertexId,
        edge: ActEdge,
    },
    MissingFanin {
        node: NodeId,
        fanin: NodeId,
    },
    ExpectedZeroFanins {
        node: NodeId,
        actual: usize,
    },
    ExpectedOneFanin {
        node: NodeId,
        actual: usize,
    },
    MissingRemap(NodeId),
}

impl fmt::Display for ActUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native SIS prerequisite ports")
            }
            Self::MissingNode(node) => write!(f, "missing node {}", node.0),
            Self::MissingNetworkNode(name) => write!(f, "missing network node {name}"),
            Self::MissingCost(node) => write!(f, "missing ACT cost for node {}", node.0),
            Self::MissingVertex(vertex) => write!(f, "missing ACT vertex {}", vertex.0),
            Self::MissingChild { vertex, edge } => {
                write!(f, "missing {edge:?} child for ACT vertex {}", vertex.0)
            }
            Self::InvalidVertexValue { vertex, value } => {
                write!(f, "invalid ACT vertex {} value {value:?}", vertex.0)
            }
            Self::InvalidTerminalChild { vertex, edge } => {
                write!(
                    f,
                    "terminal ACT vertex {} unexpectedly has a {edge:?} child",
                    vertex.0
                )
            }
            Self::InvalidInternalChild { vertex, edge } => {
                write!(f, "internal ACT vertex {} is missing {edge:?}", vertex.0)
            }
            Self::MissingFanin { node, fanin } => {
                write!(f, "node {} does not contain fanin {}", node.0, fanin.0)
            }
            Self::ExpectedZeroFanins { node, actual } => {
                write!(f, "node {} has {actual} fanins, expected zero", node.0)
            }
            Self::ExpectedOneFanin { node, actual } => {
                write!(f, "node {} has {actual} fanins, expected one", node.0)
            }
            Self::MissingRemap(node) => write!(f, "missing remap entry for node {}", node.0),
        }
    }
}

impl Error for ActUtilError {}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActNode {
    pub id: NodeId,
    pub name: String,
    pub kind: ActNodeKind,
    fanins: Vec<NodeId>,
}

impl ActNode {
    pub fn new(
        id: usize,
        name: impl Into<String>,
        kind: ActNodeKind,
        fanins: impl IntoIterator<Item = NodeId>,
    ) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind,
            fanins: fanins.into_iter().collect(),
        }
    }

    pub fn primary_input(id: usize, name: impl Into<String>) -> Self {
        Self::new(id, name, ActNodeKind::PrimaryInput, [])
    }

    pub fn internal(
        id: usize,
        name: impl Into<String>,
        fanins: impl IntoIterator<Item = NodeId>,
    ) -> Self {
        Self::new(id, name, ActNodeKind::Internal, fanins)
    }

    pub fn fanins(&self) -> &[NodeId] {
        &self.fanins
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActNetwork {
    nodes: BTreeMap<NodeId, ActNode>,
    names: BTreeMap<String, NodeId>,
}

impl ActNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: ActNode) {
        self.names.insert(node.name.clone(), node.id);
        self.nodes.insert(node.id, node);
    }

    pub fn node(&self, id: NodeId) -> ActUtilResult<&ActNode> {
        self.nodes.get(&id).ok_or(ActUtilError::MissingNode(id))
    }

    pub fn find_node(&self, name: &str) -> Option<NodeId> {
        self.names.get(name).copied()
    }

    pub fn long_name(&self, id: NodeId) -> ActUtilResult<&str> {
        self.node(id).map(|node| node.name.as_str())
    }

    pub fn nodes(&self) -> impl Iterator<Item = &ActNode> {
        self.nodes.values()
    }
}

pub fn is_fanin_of(node1: NodeId, node2: &ActNode) -> bool {
    node2.kind != ActNodeKind::PrimaryInput && node2.fanins().contains(&node1)
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActVertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActValue {
    Zero,
    One,
    Internal,
}

impl ActValue {
    pub fn from_legacy_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Zero),
            1 => Some(Self::One),
            4 => Some(Self::Internal),
            _ => None,
        }
    }

    pub const fn legacy_value(self) -> i32 {
        match self {
            Self::Zero => 0,
            Self::One => 1,
            Self::Internal => 4,
        }
    }

    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Internal)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActEdge {
    Low,
    High,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActVertex {
    pub id: ActVertexId,
    pub value: ActValue,
    pub low: Option<ActVertexId>,
    pub high: Option<ActVertexId>,
    pub index: i32,
    pub mark: i32,
    pub index_size: i32,
    pub node: Option<NodeId>,
    pub name: Option<String>,
    pub multiple_fo: usize,
    pub cost: i32,
    pub pattern_num: i32,
    pub mapped: bool,
    pub arrival_time: f64,
    pub multiple_fo_for_mapping: usize,
}

impl ActVertex {
    pub fn new(mark: i32) -> Self {
        Self {
            id: ActVertexId(usize::MAX),
            value: ActValue::Internal,
            low: None,
            high: None,
            index: 0,
            mark,
            index_size: 0,
            node: None,
            name: None,
            multiple_fo: 0,
            cost: 0,
            pattern_num: 0,
            mapped: false,
            arrival_time: 0.0,
            multiple_fo_for_mapping: 0,
        }
    }

    pub fn terminal(value: ActValue, mark: i32) -> Self {
        Self {
            value,
            ..Self::new(mark)
        }
    }

    pub fn with_children(mut self, low: ActVertexId, high: ActVertexId) -> Self {
        self.low = Some(low);
        self.high = Some(high);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VertexNode {
    pub vertex: ActVertexId,
    pub node: NodeId,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActGraph {
    vertices: Vec<ActVertex>,
    vertex_node_links: Vec<VertexNode>,
}

impl ActGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allocate_vertex(&mut self, mark: i32) -> ActVertexId {
        self.add_vertex(ActVertex::new(mark))
    }

    pub fn add_vertex(&mut self, mut vertex: ActVertex) -> ActVertexId {
        let id = ActVertexId(self.vertices.len());
        vertex.id = id;
        self.vertices.push(vertex);
        id
    }

    pub fn vertex(&self, id: ActVertexId) -> ActUtilResult<&ActVertex> {
        self.vertices
            .get(id.0)
            .ok_or(ActUtilError::MissingVertex(id))
    }

    pub fn vertex_mut(&mut self, id: ActVertexId) -> ActUtilResult<&mut ActVertex> {
        self.vertices
            .get_mut(id.0)
            .ok_or(ActUtilError::MissingVertex(id))
    }

    pub fn vertices(&self) -> &[ActVertex] {
        &self.vertices
    }

    pub fn vertex_node_links(&self) -> &[VertexNode] {
        &self.vertex_node_links
    }

    pub fn add_terminal_vertices(&mut self, vertex: ActVertexId) -> ActUtilResult<()> {
        let low = self.add_vertex(ActVertex::terminal(ActValue::Zero, MARK_COMPLEMENT_VALUE));
        let high = self.add_vertex(ActVertex::terminal(ActValue::One, MARK_COMPLEMENT_VALUE));
        let vertex = self.vertex_mut(vertex)?;
        vertex.low = Some(low);
        vertex.high = Some(high);
        Ok(())
    }

    pub fn check(&self, root: ActVertexId) -> ActUtilResult<()> {
        let mut seen = BTreeSet::new();
        self.check_vertex(root, &mut seen)
    }

    pub fn initialize_area(&mut self, root: ActVertexId) -> ActUtilResult<Vec<ActVertexId>> {
        let mut multiple_fo = Vec::new();
        self.initialize(root, Some(&mut multiple_fo))?;
        Ok(multiple_fo)
    }

    pub fn initialize_delay(&mut self, root: ActVertexId) -> ActUtilResult<()> {
        self.initialize(root, None)
    }

    pub fn make_multiple_fo_array_delay(
        &mut self,
        root: ActVertexId,
    ) -> ActUtilResult<Vec<ActVertexId>> {
        let mut multiple_fo = Vec::new();
        self.collect_multiple_fo(root, &mut multiple_fo)?;
        Ok(multiple_fo)
    }

    pub fn change_vertex_child(
        &mut self,
        vertex: ActVertexId,
        child: ActVertexId,
        edge: ActEdge,
        node: Option<&ActNode>,
    ) -> ActUtilResult<()> {
        let child_snapshot = self.vertex(child)?.clone();
        match child_snapshot.value {
            ActValue::Zero | ActValue::One => {
                if let Some(node) = node {
                    ensure_fanin_count(node, 0)?;
                }
                if child_snapshot.multiple_fo_for_mapping == 0 {
                    return Ok(());
                }
                self.vertex_mut(child)?.multiple_fo_for_mapping -= 1;
                let newchild = self.add_vertex(ActVertex::terminal(
                    child_snapshot.value,
                    MARK_COMPLEMENT_VALUE,
                ));
                self.set_child(vertex, edge, newchild)?;
                Ok(())
            }
            ActValue::Internal if self.is_direct_literal_vertex(child)? => {
                if child_snapshot.multiple_fo_for_mapping == 0 {
                    self.rewrite_shared_terminals(child)?;
                    return Ok(());
                }

                self.vertex_mut(child)?.multiple_fo_for_mapping -= 1;
                let newchild = self.add_vertex(ActVertex {
                    value: ActValue::Internal,
                    name: child_snapshot.name,
                    node: child_snapshot.node,
                    ..ActVertex::new(MARK_COMPLEMENT_VALUE)
                });
                self.set_child(vertex, edge, newchild)?;
                self.add_terminal_vertices(newchild)
            }
            ActValue::Internal => {
                if child_snapshot.multiple_fo_for_mapping != 0 {
                    self.vertex_mut(child)?.multiple_fo_for_mapping -= 1;
                }

                let newchild = self.add_vertex(ActVertex::new(MARK_COMPLEMENT_VALUE));
                self.set_child(vertex, edge, newchild)?;
                self.add_terminal_vertices(newchild)?;

                let node = node.ok_or(ActUtilError::MissingNativePorts {
                    operation: "act_change_vertex_child literal-node association",
                })?;
                ensure_fanin_count(node, 1)?;
                self.vertex_node_links.push(VertexNode {
                    vertex: newchild,
                    node: node.fanins()[0],
                });
                Ok(())
            }
        }
    }

    pub fn partial_collapse_put_node_names(
        &mut self,
        root: ActVertexId,
        network: &ActNetwork,
    ) -> ActUtilResult<()> {
        self.rewrite_names_by_network(root, network, None)
    }

    pub fn remap_put_node_names(
        &mut self,
        root: ActVertexId,
        table: &BTreeMap<NodeId, NodeId>,
        old_network: &ActNetwork,
        new_network: &ActNetwork,
    ) -> ActUtilResult<()> {
        self.rewrite_names_by_network(root, old_network, Some((table, new_network)))
    }

    pub fn traverse_descriptions(&mut self, root: ActVertexId) -> ActUtilResult<Vec<String>> {
        let mut output = Vec::new();
        self.traverse_descriptions_from(root, &mut output)?;
        Ok(output)
    }

    fn check_vertex(
        &self,
        vertex: ActVertexId,
        seen: &mut BTreeSet<ActVertexId>,
    ) -> ActUtilResult<()> {
        if !seen.insert(vertex) {
            return Ok(());
        }

        let vertex_ref = self.vertex(vertex)?;
        if vertex_ref.value.is_terminal() {
            if vertex_ref.low.is_some() {
                return Err(ActUtilError::InvalidTerminalChild {
                    vertex,
                    edge: ActEdge::Low,
                });
            }
            if vertex_ref.high.is_some() {
                return Err(ActUtilError::InvalidTerminalChild {
                    vertex,
                    edge: ActEdge::High,
                });
            }
            return Ok(());
        }

        let low = vertex_ref.low.ok_or(ActUtilError::InvalidInternalChild {
            vertex,
            edge: ActEdge::Low,
        })?;
        let high = vertex_ref.high.ok_or(ActUtilError::InvalidInternalChild {
            vertex,
            edge: ActEdge::High,
        })?;
        self.check_vertex(low, seen)?;
        self.check_vertex(high, seen)
    }

    fn initialize(
        &mut self,
        vertex: ActVertexId,
        mut multiple_fo: Option<&mut Vec<ActVertexId>>,
    ) -> ActUtilResult<()> {
        self.toggle_mark(vertex)?;
        {
            let vertex_ref = self.vertex_mut(vertex)?;
            vertex_ref.pattern_num = -1;
            vertex_ref.cost = 0;
            vertex_ref.arrival_time = 0.0;
            vertex_ref.mapped = false;
            vertex_ref.multiple_fo = 0;
            vertex_ref.multiple_fo_for_mapping = 0;
            if vertex_ref.value.is_terminal() {
                return Ok(());
            }
        }

        for edge in [ActEdge::Low, ActEdge::High] {
            let child = self.required_child(vertex, edge)?;
            if self.vertex(vertex)?.mark != self.vertex(child)?.mark {
                self.initialize(child, multiple_fo.as_deref_mut())?;
            } else {
                let child_ref = self.vertex_mut(child)?;
                if let Some(multiple_fo) = multiple_fo.as_deref_mut() {
                    if child_ref.multiple_fo == 0 {
                        multiple_fo.push(child);
                    }
                }
                child_ref.multiple_fo += 1;
                child_ref.multiple_fo_for_mapping += 1;
            }
        }
        Ok(())
    }

    fn collect_multiple_fo(
        &mut self,
        vertex: ActVertexId,
        multiple_fo: &mut Vec<ActVertexId>,
    ) -> ActUtilResult<()> {
        self.toggle_mark(vertex)?;
        if self.vertex(vertex)?.value.is_terminal() {
            return Ok(());
        }
        if self.vertex(vertex)?.multiple_fo != 0 {
            insert_unique(vertex, multiple_fo);
        }

        for edge in [ActEdge::Low, ActEdge::High] {
            let child = self.required_child(vertex, edge)?;
            if self.vertex(vertex)?.mark != self.vertex(child)?.mark {
                self.collect_multiple_fo(child, multiple_fo)?;
            }
        }
        Ok(())
    }

    fn rewrite_names_by_network(
        &mut self,
        vertex: ActVertexId,
        lookup_network: &ActNetwork,
        remap: Option<(&BTreeMap<NodeId, NodeId>, &ActNetwork)>,
    ) -> ActUtilResult<()> {
        self.toggle_mark(vertex)?;
        let snapshot = self.vertex(vertex)?.clone();
        if snapshot.value.is_terminal() {
            return Ok(());
        }

        let name = snapshot.name.ok_or(ActUtilError::MissingNativePorts {
            operation: "ACT vertex name lookup",
        })?;
        let old_node = lookup_network
            .find_node(&name)
            .ok_or_else(|| ActUtilError::MissingNetworkNode(name.clone()))?;
        let replacement_node = match remap {
            Some((table, _)) => *table
                .get(&old_node)
                .ok_or(ActUtilError::MissingRemap(old_node))?,
            None => old_node,
        };
        let name_network = remap.map(|(_, network)| network).unwrap_or(lookup_network);
        self.vertex_mut(vertex)?.name = Some(name_network.long_name(replacement_node)?.to_owned());
        self.vertex_mut(vertex)?.node = Some(replacement_node);

        for edge in [ActEdge::Low, ActEdge::High] {
            let child = self.required_child(vertex, edge)?;
            if self.vertex(vertex)?.mark != self.vertex(child)?.mark {
                self.rewrite_names_by_network(child, lookup_network, remap)?;
            }
        }
        Ok(())
    }

    fn traverse_descriptions_from(
        &mut self,
        vertex: ActVertexId,
        output: &mut Vec<String>,
    ) -> ActUtilResult<()> {
        self.toggle_mark(vertex)?;
        output.push(self.vertex_description(vertex)?);
        if self.vertex(vertex)?.value.is_terminal() {
            return Ok(());
        }

        let low = self.required_child(vertex, ActEdge::Low)?;
        let high = self.required_child(vertex, ActEdge::High)?;
        if self.vertex(vertex)?.mark == self.vertex(low)?.mark {
            output.push(self.vertex_description(low)?);
            if self.vertex(vertex)?.mark == self.vertex(high)?.mark {
                output.push(self.vertex_description(high)?);
                return Ok(());
            }
            self.traverse_descriptions_from(high, output)?;
            return Ok(());
        }
        self.traverse_descriptions_from(low, output)?;
        if self.vertex(vertex)?.mark == self.vertex(high)?.mark {
            output.push(self.vertex_description(high)?);
            return Ok(());
        }
        self.traverse_descriptions_from(high, output)
    }

    fn vertex_description(&self, vertex: ActVertexId) -> ActUtilResult<String> {
        let vertex_ref = self.vertex(vertex)?;
        if vertex_ref.value.is_terminal() {
            return Ok(format!(
                "value = {}, id = {}, index = {}, multiple_fo_for_mapping = {}",
                vertex_ref.value.legacy_value(),
                vertex_ref.id.0,
                vertex_ref.index,
                vertex_ref.multiple_fo_for_mapping
            ));
        }

        match &vertex_ref.name {
            Some(name) => Ok(format!(
                "name = {name}, index = {}, num_fanouts = {}, multiple_fo_for_mapping = {}",
                vertex_ref.index,
                vertex_ref.multiple_fo + 1,
                vertex_ref.multiple_fo_for_mapping
            )),
            None => Ok(format!(
                "id = {}, index = {}, num_fanouts = {}, multiple_fo_for_mapping = {}",
                vertex_ref.id.0,
                vertex_ref.index,
                vertex_ref.multiple_fo + 1,
                vertex_ref.multiple_fo_for_mapping
            )),
        }
    }

    fn rewrite_shared_terminals(&mut self, child: ActVertexId) -> ActUtilResult<()> {
        for edge in [ActEdge::Low, ActEdge::High] {
            let terminal = self.required_child(child, edge)?;
            if self.vertex(terminal)?.multiple_fo_for_mapping == 0 {
                continue;
            }

            self.vertex_mut(terminal)?.multiple_fo_for_mapping -= 1;
            let value = self.vertex(terminal)?.value;
            let replacement = self.add_vertex(ActVertex::terminal(value, MARK_COMPLEMENT_VALUE));
            self.set_child(child, edge, replacement)?;
        }
        Ok(())
    }

    fn is_direct_literal_vertex(&self, vertex: ActVertexId) -> ActUtilResult<bool> {
        let low = self.required_child(vertex, ActEdge::Low)?;
        let high = self.required_child(vertex, ActEdge::High)?;
        Ok(self.vertex(low)?.value == ActValue::Zero && self.vertex(high)?.value == ActValue::One)
    }

    fn set_child(
        &mut self,
        vertex: ActVertexId,
        edge: ActEdge,
        child: ActVertexId,
    ) -> ActUtilResult<()> {
        let vertex_ref = self.vertex_mut(vertex)?;
        match edge {
            ActEdge::Low => vertex_ref.low = Some(child),
            ActEdge::High => vertex_ref.high = Some(child),
        }
        Ok(())
    }

    fn required_child(&self, vertex: ActVertexId, edge: ActEdge) -> ActUtilResult<ActVertexId> {
        let vertex_ref = self.vertex(vertex)?;
        match edge {
            ActEdge::Low => vertex_ref.low,
            ActEdge::High => vertex_ref.high,
        }
        .ok_or(ActUtilError::MissingChild { vertex, edge })
    }

    fn toggle_mark(&mut self, vertex: ActVertexId) -> ActUtilResult<()> {
        let vertex_ref = self.vertex_mut(vertex)?;
        vertex_ref.mark = if vertex_ref.mark == 0 { 1 } else { 0 };
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CostStruct {
    pub node: NodeId,
    pub cost: i32,
    pub arrival_time: f64,
    pub required_time: f64,
    pub slack: f64,
    pub is_critical: bool,
    pub area_weight: f64,
    pub cost_and_arrival_time: f64,
    pub act: ActVertexId,
}

pub fn allocate_cost_node(
    node: NodeId,
    cost: i32,
    arrival_time: f64,
    required_time: f64,
    slack: f64,
    is_critical: bool,
    area_weight: f64,
    cost_and_arrival_time: f64,
    act: ActVertexId,
) -> CostStruct {
    CostStruct {
        node,
        cost,
        arrival_time,
        required_time,
        slack,
        is_critical,
        area_weight,
        cost_and_arrival_time,
        act,
    }
}

pub fn final_check_network(
    network: &ActNetwork,
    cost_table: &BTreeMap<NodeId, CostStruct>,
    graph: &ActGraph,
) -> ActUtilResult<()> {
    for node in network.nodes() {
        final_check_node(node, cost_table, graph)?;
    }
    Ok(())
}

pub fn final_check_node(
    node: &ActNode,
    cost_table: &BTreeMap<NodeId, CostStruct>,
    graph: &ActGraph,
) -> ActUtilResult<()> {
    if node.kind != ActNodeKind::Internal {
        return Ok(());
    }
    let cost = cost_table
        .get(&node.id)
        .ok_or(ActUtilError::MissingCost(node.id))?;
    graph.check(cost.act)
}

pub fn max_f64(a: f64, b: f64) -> f64 {
    if a >= b { a } else { b }
}

pub fn allocate_vertex_node_struct(vertex: ActVertexId, node: NodeId) -> VertexNode {
    VertexNode { vertex, node }
}

pub fn put_nodes(node: &ActNode, excluded_fanin: NodeId) -> ActUtilResult<Vec<NodeId>> {
    let mut result = Vec::new();
    let mut found = false;
    for fanin in node.fanins().iter().rev().copied() {
        if fanin == excluded_fanin {
            found = true;
            continue;
        }
        result.push(fanin);
    }

    if !found {
        return Err(ActUtilError::MissingFanin {
            node: node.id,
            fanin: excluded_fanin,
        });
    }
    Ok(result)
}

pub fn insert_unique(vertex: ActVertexId, vertices: &mut Vec<ActVertexId>) -> bool {
    if vertices.contains(&vertex) {
        return false;
    }
    vertices.push(vertex);
    true
}

pub fn partial_collapse_update_act_fields(
    network: &ActNetwork,
    duplicate_fanout: &ActNode,
    cost: &mut CostStruct,
    graph: &mut ActGraph,
) -> ActUtilResult<()> {
    let fanout = network
        .find_node(&duplicate_fanout.name)
        .ok_or_else(|| ActUtilError::MissingNetworkNode(duplicate_fanout.name.clone()))?;
    cost.node = fanout;
    graph.vertex_mut(cost.act)?.node = Some(fanout);
    graph.partial_collapse_put_node_names(cost.act, network)
}

pub fn remap_update_act_fields(
    table: &BTreeMap<NodeId, NodeId>,
    old_network: &ActNetwork,
    new_node: NodeId,
    new_network: &ActNetwork,
    cost: &mut CostStruct,
    graph: &mut ActGraph,
) -> ActUtilResult<()> {
    cost.node = new_node;
    graph.vertex_mut(cost.act)?.node = Some(new_node);
    graph.remap_put_node_names(cost.act, table, old_network, new_network)
}

pub fn sis_bound_operation_unavailable(operation: &'static str) -> ActUtilResult<()> {
    Err(ActUtilError::MissingNativePorts { operation })
}

fn ensure_fanin_count(node: &ActNode, expected: usize) -> ActUtilResult<()> {
    let actual = node.fanins().len();
    if actual == expected {
        return Ok(());
    }

    match expected {
        0 => Err(ActUtilError::ExpectedZeroFanins {
            node: node.id,
            actual,
        }),
        1 => Err(ActUtilError::ExpectedOneFanin {
            node: node.id,
            actual,
        }),
        _ => Err(ActUtilError::MissingNativePorts {
            operation: "ACT fanin count validation",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn network_with(nodes: &[ActNode]) -> ActNetwork {
        let mut network = ActNetwork::new();
        for node in nodes {
            network.add_node(node.clone());
        }
        network
    }

    fn literal_graph() -> (ActGraph, ActVertexId) {
        let mut graph = ActGraph::new();
        let root = graph.allocate_vertex(1);
        graph.add_terminal_vertices(root).unwrap();
        (graph, root)
    }

    #[test]
    fn is_fanin_of_rejects_primary_inputs_and_matches_internal_fanins() {
        let pi = ActNode::primary_input(1, "a");
        let internal = ActNode::internal(2, "n", [NodeId(1), NodeId(3)]);

        assert!(!is_fanin_of(NodeId(1), &pi));
        assert!(is_fanin_of(NodeId(1), &internal));
        assert!(!is_fanin_of(NodeId(4), &internal));
    }

    #[test]
    fn allocate_cost_node_copies_timing_and_mapping_fields() {
        let cost = allocate_cost_node(NodeId(1), 7, 1.5, 3.0, 1.5, true, 0.75, 8.5, ActVertexId(2));

        assert_eq!(cost.node, NodeId(1));
        assert_eq!(cost.cost, 7);
        assert_eq!(cost.arrival_time, 1.5);
        assert_eq!(cost.required_time, 3.0);
        assert_eq!(cost.slack, 1.5);
        assert!(cost.is_critical);
        assert_eq!(cost.area_weight, 0.75);
        assert_eq!(cost.cost_and_arrival_time, 8.5);
        assert_eq!(cost.act, ActVertexId(2));
    }

    #[test]
    fn check_validates_terminal_and_internal_child_shape() {
        let (mut graph, root) = literal_graph();
        graph.check(root).unwrap();

        let low = graph.vertex(root).unwrap().low.unwrap();
        graph.vertex_mut(low).unwrap().low = Some(root);
        assert!(matches!(
            graph.check(root),
            Err(ActUtilError::InvalidTerminalChild {
                vertex,
                edge: ActEdge::Low
            }) if vertex == low
        ));
    }

    #[test]
    fn initialize_area_resets_fields_and_records_shared_vertices_once() {
        let mut graph = ActGraph::new();
        let shared = graph.add_vertex(ActVertex {
            cost: 9,
            multiple_fo: 5,
            multiple_fo_for_mapping: 5,
            ..ActVertex::terminal(ActValue::One, 1)
        });
        let root = graph.add_vertex(ActVertex::new(1).with_children(shared, shared));

        let multiple_fo = graph.initialize_area(root).unwrap();

        assert_eq!(multiple_fo, vec![shared]);
        assert_eq!(graph.vertex(shared).unwrap().multiple_fo, 1);
        assert_eq!(graph.vertex(shared).unwrap().multiple_fo_for_mapping, 1);
        assert_eq!(graph.vertex(root).unwrap().pattern_num, -1);
    }

    #[test]
    fn initialize_delay_counts_shared_vertices_without_collecting_array() {
        let mut graph = ActGraph::new();
        let zero = graph.add_vertex(ActVertex::terminal(ActValue::Zero, 1));
        let one = graph.add_vertex(ActVertex::terminal(ActValue::One, 1));
        let shared = graph.add_vertex(ActVertex::new(1).with_children(zero, one));
        let root = graph.add_vertex(ActVertex::new(1).with_children(shared, shared));

        graph.initialize_delay(root).unwrap();

        assert_eq!(graph.vertex(shared).unwrap().multiple_fo, 1);
        assert_eq!(
            graph.make_multiple_fo_array_delay(root).unwrap(),
            vec![shared]
        );
    }

    #[test]
    fn change_vertex_child_copies_shared_terminal_child() {
        let mut graph = ActGraph::new();
        let terminal = graph.add_vertex(ActVertex {
            multiple_fo_for_mapping: 2,
            ..ActVertex::terminal(ActValue::Zero, 1)
        });
        let parent = graph.add_vertex(ActVertex::new(1).with_children(terminal, terminal));
        let literal_node = ActNode::internal(9, "literal", []);

        graph
            .change_vertex_child(parent, terminal, ActEdge::Low, Some(&literal_node))
            .unwrap();

        let new_low = graph.vertex(parent).unwrap().low.unwrap();
        assert_ne!(new_low, terminal);
        assert_eq!(graph.vertex(new_low).unwrap().value, ActValue::Zero);
        assert_eq!(graph.vertex(terminal).unwrap().multiple_fo_for_mapping, 1);
    }

    #[test]
    fn change_vertex_child_copies_shared_direct_literal_vertex_with_terminals() {
        let (mut graph, child) = literal_graph();
        graph.vertex_mut(child).unwrap().multiple_fo_for_mapping = 1;
        graph.vertex_mut(child).unwrap().name = Some("x".to_owned());
        let parent = graph.add_vertex(ActVertex::new(1).with_children(child, child));

        graph
            .change_vertex_child(parent, child, ActEdge::High, None)
            .unwrap();

        let new_high = graph.vertex(parent).unwrap().high.unwrap();
        assert_ne!(new_high, child);
        assert_eq!(graph.vertex(child).unwrap().multiple_fo_for_mapping, 0);
        assert_eq!(graph.vertex(new_high).unwrap().name.as_deref(), Some("x"));
        assert!(graph.vertex(new_high).unwrap().low.is_some());
        assert!(graph.vertex(new_high).unwrap().high.is_some());
    }

    #[test]
    fn change_vertex_child_creates_literal_vertex_node_link_for_nontrivial_child() {
        let mut graph = ActGraph::new();
        let zero = graph.add_vertex(ActVertex::terminal(ActValue::Zero, 1));
        let one = graph.add_vertex(ActVertex::terminal(ActValue::One, 1));
        let nontrivial_child = graph.add_vertex(ActVertex::new(1).with_children(one, zero));
        let parent = graph.add_vertex(ActVertex::new(1).with_children(nontrivial_child, zero));
        let literal_node = ActNode::internal(7, "lit", [NodeId(99)]);

        graph
            .change_vertex_child(parent, nontrivial_child, ActEdge::Low, Some(&literal_node))
            .unwrap();

        let new_low = graph.vertex(parent).unwrap().low.unwrap();
        assert_ne!(new_low, nontrivial_child);
        assert_eq!(
            graph.vertex_node_links(),
            &[VertexNode {
                vertex: new_low,
                node: NodeId(99)
            }]
        );
    }

    #[test]
    fn put_nodes_returns_reverse_fanins_except_selected_fanin() {
        let node = ActNode::internal(4, "n", [NodeId(1), NodeId(2), NodeId(3)]);

        assert_eq!(
            put_nodes(&node, NodeId(2)).unwrap(),
            vec![NodeId(3), NodeId(1)]
        );
        assert!(matches!(
            put_nodes(&node, NodeId(9)),
            Err(ActUtilError::MissingFanin { .. })
        ));
    }

    #[test]
    fn partial_collapse_put_node_names_rebinds_names_to_network_owned_names() {
        let old = ActNode::internal(1, "old", []);
        let network = network_with(&[old]);
        let (mut graph, root) = literal_graph();
        graph.vertex_mut(root).unwrap().name = Some("old".to_owned());

        graph
            .partial_collapse_put_node_names(root, &network)
            .unwrap();

        assert_eq!(graph.vertex(root).unwrap().node, Some(NodeId(1)));
        assert_eq!(graph.vertex(root).unwrap().name.as_deref(), Some("old"));
    }

    #[test]
    fn remap_put_node_names_uses_table_and_new_network_names() {
        let old_network = network_with(&[ActNode::internal(1, "old", [])]);
        let new_network = network_with(&[ActNode::internal(2, "new", [])]);
        let table = BTreeMap::from([(NodeId(1), NodeId(2))]);
        let (mut graph, root) = literal_graph();
        graph.vertex_mut(root).unwrap().name = Some("old".to_owned());

        graph
            .remap_put_node_names(root, &table, &old_network, &new_network)
            .unwrap();

        assert_eq!(graph.vertex(root).unwrap().node, Some(NodeId(2)));
        assert_eq!(graph.vertex(root).unwrap().name.as_deref(), Some("new"));
    }

    #[test]
    fn final_check_network_checks_only_internal_costed_nodes() {
        let network = network_with(&[
            ActNode::primary_input(1, "a"),
            ActNode::internal(2, "n", [NodeId(1)]),
        ]);
        let (graph, root) = literal_graph();
        let cost_table = BTreeMap::from([(
            NodeId(2),
            allocate_cost_node(NodeId(2), 1, 0.0, 0.0, 0.0, false, 0.0, 0.0, root),
        )]);

        final_check_network(&network, &cost_table, &graph).unwrap();
    }

    #[test]
    fn traverse_descriptions_matches_low_then_high_marked_traversal_shape() {
        let (mut graph, root) = literal_graph();
        graph.vertex_mut(root).unwrap().name = Some("n".to_owned());

        let descriptions = graph.traverse_descriptions(root).unwrap();

        assert_eq!(descriptions.len(), 3);
        assert!(descriptions[0].contains("name = n"));
        assert!(descriptions[1].contains("value = 0"));
        assert!(descriptions[2].contains("value = 1"));
    }

    #[test]
    fn no_per_file_c_abi_or_task_metadata_tokens_are_present() {
        let source = include_str!("act_util.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("be", "ad")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
