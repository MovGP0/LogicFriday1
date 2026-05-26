//! Native Rust model for `LogicSynthesis/sis/pld/ite_util.c`.
//!
//! The C file is mostly ownership and pointer-graph utility code around SIS
//! `node_t`, `network_t`, ACT, ITE, and `st_table` storage. This port keeps the
//! local behavior in owned Rust data structures and reports direct SIS-backed
//! entry points as explicit dependency errors until those native ports exist.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

pub const MARK_COMPLEMENT_VALUE: i32 = 0;

pub fn sis_bound_operation_unavailable(operation: &'static str) -> Result<(), IteUtilError> {
    Err(IteUtilError::MissingNativePorts { operation })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IteUtilError {
    MissingNativePorts {
        operation: &'static str,
    },
    MissingMatch {
        node: NodeId,
        map_alg: i32,
    },
    MissingNode(NodeId),
    MissingNetworkNode(String),
    MissingRemap(NodeId),
    MissingIteVertex(IteVertexId),
    MissingActVertex(ActVertexId),
    ExpectedLiteral {
        vertex: IteVertexId,
        value: IteValue,
    },
    ExpectedIteOrAct {
        node: NodeId,
    },
}

impl fmt::Display for IteUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native SIS prerequisite ports")
            }
            Self::MissingMatch { node, map_alg } => {
                write!(
                    f,
                    "no ACT match for node {} with map algorithm {map_alg}",
                    node.0
                )
            }
            Self::MissingNode(node) => write!(f, "missing node {}", node.0),
            Self::MissingNetworkNode(name) => write!(f, "missing network node {name}"),
            Self::MissingRemap(node) => write!(f, "missing remap entry for node {}", node.0),
            Self::MissingIteVertex(vertex) => write!(f, "missing ITE vertex {}", vertex.0),
            Self::MissingActVertex(vertex) => write!(f, "missing ACT vertex {}", vertex.0),
            Self::ExpectedLiteral { vertex, value } => {
                write!(
                    f,
                    "ITE vertex {} has value {value:?}, expected literal",
                    vertex.0
                )
            }
            Self::ExpectedIteOrAct { node } => {
                write!(
                    f,
                    "cost slot for node {} has neither ITE nor ACT graph",
                    node.0
                )
            }
        }
    }
}

impl Error for IteUtilError {}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SisNode {
    pub id: NodeId,
    pub name: String,
    pub literal_phase: Option<bool>,
    pub cost: Option<ActIteCostStruct>,
    pub freed: bool,
}

impl SisNode {
    pub fn new(id: usize, name: impl Into<String>) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            literal_phase: None,
            cost: None,
            freed: false,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SisNetwork {
    nodes: HashMap<NodeId, SisNode>,
    names: HashMap<String, NodeId>,
    pub freed: bool,
}

impl SisNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: SisNode) {
        self.names.insert(node.name.clone(), node.id);
        self.nodes.insert(node.id, node);
    }

    pub fn find_node(&self, name: &str) -> Option<NodeId> {
        self.names.get(name).copied()
    }

    pub fn long_name(&self, node: NodeId) -> Result<&str, IteUtilError> {
        self.nodes
            .get(&node)
            .map(|node| node.name.as_str())
            .ok_or(IteUtilError::MissingNode(node))
    }

    pub fn node(&self, node: NodeId) -> Option<&SisNode> {
        self.nodes.get(&node)
    }

    pub fn node_mut(&mut self, node: NodeId) -> Option<&mut SisNode> {
        self.nodes.get_mut(&node)
    }

    pub fn free_node(&mut self, node: NodeId) -> Result<(), IteUtilError> {
        let node_ref = self
            .nodes
            .get_mut(&node)
            .ok_or(IteUtilError::MissingNode(node))?;
        node_ref.freed = true;
        Ok(())
    }

    pub fn free(&mut self) {
        self.freed = true;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeLiteral {
    pub node: NodeId,
    pub phase: bool,
}

pub fn ite_get_node_literal_of_vertex(
    graph: &IteGraph,
    vertex: IteVertexId,
) -> Result<NodeLiteral, IteUtilError> {
    let ite_vertex = graph.vertex(vertex)?;
    if ite_vertex.value != IteValue::Literal {
        return Err(IteUtilError::ExpectedLiteral {
            vertex,
            value: ite_vertex.value,
        });
    }
    let node = ite_vertex
        .fanin
        .ok_or(IteUtilError::MissingIteVertex(vertex))?;
    Ok(NodeLiteral { node, phase: true })
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IteVertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IteValue {
    Zero,
    One,
    Literal,
    IfThenElse,
}

impl IteValue {
    fn from_c_value(value: i32) -> Self {
        match value {
            0 => Self::Zero,
            1 => Self::One,
            2 => Self::Literal,
            _ => Self::IfThenElse,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutIte {
    pub next_present: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IteVertex {
    pub id: IteVertexId,
    pub value: IteValue,
    pub if_child: Option<IteVertexId>,
    pub then_child: Option<IteVertexId>,
    pub else_child: Option<IteVertexId>,
    pub fanout: Option<FanoutIte>,
    pub node: Option<NodeId>,
    pub fanin: Option<NodeId>,
    pub name: Option<String>,
    pub mark: i32,
    pub multiple_fo: bool,
    pub freed: bool,
}

impl IteVertex {
    pub fn new(id: usize, value: IteValue) -> Self {
        Self {
            id: IteVertexId(id),
            value,
            if_child: None,
            then_child: None,
            else_child: None,
            fanout: None,
            node: None,
            fanin: None,
            name: None,
            mark: 1,
            multiple_fo: false,
            freed: false,
        }
    }

    pub fn c_value(id: usize, value: i32) -> Self {
        Self::new(id, IteValue::from_c_value(value))
    }

    pub fn with_children(
        mut self,
        if_child: IteVertexId,
        then_child: IteVertexId,
        else_child: IteVertexId,
    ) -> Self {
        self.if_child = Some(if_child);
        self.then_child = Some(then_child);
        self.else_child = Some(else_child);
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IteGraph {
    vertices: Vec<IteVertex>,
}

impl IteGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_vertex(&mut self, mut vertex: IteVertex) -> IteVertexId {
        let id = IteVertexId(self.vertices.len());
        vertex.id = id;
        self.vertices.push(vertex);
        id
    }

    pub fn vertex(&self, id: IteVertexId) -> Result<&IteVertex, IteUtilError> {
        self.vertices
            .get(id.0)
            .ok_or(IteUtilError::MissingIteVertex(id))
    }

    pub fn vertex_mut(&mut self, id: IteVertexId) -> Result<&mut IteVertex, IteUtilError> {
        self.vertices
            .get_mut(id.0)
            .ok_or(IteUtilError::MissingIteVertex(id))
    }

    pub fn traverse_ite(&self, root: IteVertexId) -> Result<Vec<IteVertexId>, IteUtilError> {
        let mut seen = HashSet::new();
        let mut order = Vec::new();
        self.traverse_from(root, &mut seen, &mut order)?;
        Ok(order)
    }

    pub fn free_from(&mut self, root: IteVertexId) -> Result<FreeReport, IteUtilError> {
        let order = self.traverse_ite(root)?;
        let mut report = FreeReport::default();
        for id in order {
            let vertex = self.vertex_mut(id)?;
            report.vertices += usize::from(!vertex.freed);
            if let Some(fanout) = vertex.fanout.take() {
                report.fanouts += 1;
                report.had_non_nil_fanout_next |= fanout.next_present;
            }
            vertex.freed = true;
        }
        Ok(report)
    }

    pub fn remap_literal_nodes(
        &mut self,
        root: IteVertexId,
        remap: &HashMap<NodeId, NodeId>,
        old_network: &SisNetwork,
        new_network: &SisNetwork,
    ) -> Result<(), IteUtilError> {
        for id in self.traverse_ite(root)? {
            let fanin = match self.vertex(id)? {
                IteVertex {
                    value: IteValue::Literal,
                    fanin: Some(fanin),
                    ..
                } => *fanin,
                _ => continue,
            };
            let old_name = old_network.long_name(fanin)?;
            let old_node = old_network
                .find_node(old_name)
                .ok_or_else(|| IteUtilError::MissingNetworkNode(old_name.to_owned()))?;
            let new_node = *remap
                .get(&old_node)
                .ok_or(IteUtilError::MissingRemap(old_node))?;
            let new_name = new_network.long_name(new_node)?.to_owned();
            let vertex = self.vertex_mut(id)?;
            vertex.name = Some(new_name);
            vertex.fanin = Some(new_node);
        }
        Ok(())
    }

    pub fn free_nodes_in_multiple_fo_ite(
        &mut self,
        root: IteVertexId,
        network: &mut SisNetwork,
    ) -> Result<usize, IteUtilError> {
        let mut freed = 0;
        self.free_multiple_fo_from(root, network, &mut freed)?;
        Ok(freed)
    }

    fn traverse_from(
        &self,
        id: IteVertexId,
        seen: &mut HashSet<IteVertexId>,
        order: &mut Vec<IteVertexId>,
    ) -> Result<(), IteUtilError> {
        if !seen.insert(id) {
            return Ok(());
        }
        order.push(id);
        let vertex = self.vertex(id)?;
        if vertex.value != IteValue::IfThenElse {
            return Ok(());
        }
        for child in [vertex.if_child, vertex.then_child, vertex.else_child]
            .into_iter()
            .flatten()
        {
            self.traverse_from(child, seen, order)?;
        }
        Ok(())
    }

    fn free_multiple_fo_from(
        &mut self,
        id: IteVertexId,
        network: &mut SisNetwork,
        freed: &mut usize,
    ) -> Result<(), IteUtilError> {
        {
            let vertex = self.vertex_mut(id)?;
            if vertex.mark == MARK_COMPLEMENT_VALUE {
                return Ok(());
            }
            vertex.mark = MARK_COMPLEMENT_VALUE;
            if matches!(vertex.value, IteValue::Zero | IteValue::One) {
                return Ok(());
            }
            if vertex.multiple_fo {
                if let Some(node) = vertex.node {
                    network.free_node(node)?;
                    *freed += 1;
                }
            }
            if vertex.value != IteValue::IfThenElse {
                return Ok(());
            }
        }

        let vertex = self.vertex(id)?.clone();
        for child in [vertex.if_child, vertex.then_child, vertex.else_child]
            .into_iter()
            .flatten()
        {
            self.free_multiple_fo_from(child, network, freed)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FreeReport {
    pub vertices: usize,
    pub fanouts: usize,
    pub had_non_nil_fanout_next: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActVertexId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActVertex {
    pub id: ActVertexId,
    pub value: i32,
    pub low: Option<ActVertexId>,
    pub high: Option<ActVertexId>,
    pub node: Option<NodeId>,
    pub mark: i32,
    pub multiple_fo: bool,
    pub freed: bool,
}

impl ActVertex {
    pub fn new(id: usize, value: i32) -> Self {
        Self {
            id: ActVertexId(id),
            value,
            low: None,
            high: None,
            node: None,
            mark: 1,
            multiple_fo: false,
            freed: false,
        }
    }

    pub fn with_children(mut self, low: ActVertexId, high: ActVertexId) -> Self {
        self.low = Some(low);
        self.high = Some(high);
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActGraph {
    vertices: Vec<ActVertex>,
}

impl ActGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_vertex(&mut self, mut vertex: ActVertex) -> ActVertexId {
        let id = ActVertexId(self.vertices.len());
        vertex.id = id;
        self.vertices.push(vertex);
        id
    }

    pub fn vertex(&self, id: ActVertexId) -> Result<&ActVertex, IteUtilError> {
        self.vertices
            .get(id.0)
            .ok_or(IteUtilError::MissingActVertex(id))
    }

    pub fn vertex_mut(&mut self, id: ActVertexId) -> Result<&mut ActVertex, IteUtilError> {
        self.vertices
            .get_mut(id.0)
            .ok_or(IteUtilError::MissingActVertex(id))
    }

    pub fn free_nodes_in_multiple_fo_act(
        &mut self,
        root: ActVertexId,
        network: &mut SisNetwork,
    ) -> Result<usize, IteUtilError> {
        let mut freed = 0;
        self.free_multiple_fo_from(root, network, &mut freed)?;
        Ok(freed)
    }

    fn free_multiple_fo_from(
        &mut self,
        id: ActVertexId,
        network: &mut SisNetwork,
        freed: &mut usize,
    ) -> Result<(), IteUtilError> {
        {
            let vertex = self.vertex_mut(id)?;
            if vertex.mark == MARK_COMPLEMENT_VALUE {
                return Ok(());
            }
            vertex.mark = MARK_COMPLEMENT_VALUE;
            if vertex.value <= 1 {
                return Ok(());
            }
            if vertex.multiple_fo {
                if let Some(node) = vertex.node {
                    network.free_node(node)?;
                    *freed += 1;
                }
            }
        }

        let vertex = self.vertex(id)?.clone();
        for child in [vertex.low, vertex.high].into_iter().flatten() {
            self.free_multiple_fo_from(child, network, freed)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActMatch {
    pub a0: Option<NodeId>,
    pub a1: Option<NodeId>,
    pub sa: Option<NodeId>,
    pub b0: Option<NodeId>,
    pub b1: Option<NodeId>,
    pub sb: Option<NodeId>,
    pub s0: Option<NodeId>,
    pub s1: Option<NodeId>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActIteCostStruct {
    pub node: Option<NodeId>,
    pub ite: Option<IteGraph>,
    pub ite_root: Option<IteVertexId>,
    pub act: Option<ActGraph>,
    pub act_root: Option<ActVertexId>,
    pub match_info: Option<ActMatch>,
    pub network: Option<SisNetwork>,
}

impl ActIteCostStruct {
    pub fn free_owned_fields(&mut self) -> Result<FreeReport, IteUtilError> {
        let report = match (self.ite.as_mut(), self.ite_root) {
            (Some(graph), Some(root)) => graph.free_from(root)?,
            _ => FreeReport::default(),
        };
        self.ite = None;
        self.ite_root = None;
        self.act = None;
        self.act_root = None;
        self.match_info = None;
        if let Some(network) = self.network.as_mut() {
            network.free();
        }
        self.network = None;
        Ok(report)
    }
}

pub fn act_free_ite_node(node: &mut SisNode) -> Result<Option<FreeReport>, IteUtilError> {
    match node.cost.as_mut() {
        Some(cost) => cost.free_owned_fields().map(Some),
        None => Ok(None),
    }
}

pub fn act_free_ite_network(network: &mut SisNetwork) -> Result<usize, IteUtilError> {
    let node_ids = network.nodes.keys().copied().collect::<Vec<_>>();
    let mut freed_slots = 0;
    for id in node_ids {
        if let Some(node) = network.node_mut(id) {
            if act_free_ite_node(node)?.is_some() {
                freed_slots += 1;
            }
        }
    }
    Ok(freed_slots)
}

pub fn act_ite_cost_struct_replace_with_matcher<F>(
    n1: &mut SisNode,
    n2: &mut SisNode,
    map_alg: i32,
    mut matcher: F,
) -> Result<(), IteUtilError>
where
    F: FnMut(NodeId, i32) -> Option<ActMatch>,
{
    if let Some(mut old_cost) = n1.cost.take() {
        old_cost.free_owned_fields()?;
    }

    n1.cost = n2.cost.take();
    if let Some(cost) = n1.cost.as_mut() {
        if cost.match_info.is_some() {
            cost.match_info = matcher(n1.id, map_alg);
            if cost.match_info.is_none() {
                return Err(IteUtilError::MissingMatch {
                    node: n1.id,
                    map_alg,
                });
            }
        }
    }
    Ok(())
}

pub fn act_ite_cost_struct_replace_blocked(
    _n1: &mut SisNode,
    _n2: &mut SisNode,
    _map_alg: i32,
) -> Result<(), IteUtilError> {
    sis_bound_operation_unavailable("act_ite_cost_struct_replace")
}

pub fn act_ite_remap_update_ite_fields(
    remap: &HashMap<NodeId, NodeId>,
    old_network: &SisNetwork,
    new_network: &SisNetwork,
    node: &mut SisNode,
) -> Result<(), IteUtilError> {
    let cost = node
        .cost
        .as_mut()
        .ok_or(IteUtilError::ExpectedIteOrAct { node: node.id })?;
    cost.node = Some(node.id);
    if cost.match_info.is_some() {
        return Ok(());
    }
    if let (Some(graph), Some(root)) = (cost.ite.as_mut(), cost.ite_root) {
        graph.vertex_mut(root)?.node = Some(node.id);
        graph.remap_literal_nodes(root, remap, old_network, new_network)?;
        if cost.act.is_some() {
            return Err(IteUtilError::ExpectedIteOrAct { node: node.id });
        }
        return Ok(());
    }
    if let (Some(graph), Some(root)) = (cost.act.as_mut(), cost.act_root) {
        graph.vertex_mut(root)?.node = Some(node.id);
        return sis_bound_operation_unavailable("act_remap_put_node_names_in_act");
    }
    Err(IteUtilError::ExpectedIteOrAct { node: node.id })
}

pub fn act_ite_remap_put_node_names_in_ite(
    graph: &mut IteGraph,
    root: IteVertexId,
    remap: &HashMap<NodeId, NodeId>,
    old_network: &SisNetwork,
    new_network: &SisNetwork,
) -> Result<(), IteUtilError> {
    graph.remap_literal_nodes(root, remap, old_network, new_network)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn network_with(nodes: &[SisNode]) -> SisNetwork {
        let mut network = SisNetwork::new();
        for node in nodes {
            network.add_node(node.clone());
        }
        network
    }

    #[test]
    fn traverse_ite_visits_shared_vertices_once() {
        let mut graph = IteGraph::new();
        let lit = graph.add_vertex(IteVertex::c_value(0, 2));
        let one = graph.add_vertex(IteVertex::c_value(0, 1));
        let root = graph.add_vertex(IteVertex::c_value(0, 3).with_children(lit, one, lit));

        assert_eq!(graph.traverse_ite(root).unwrap(), vec![root, lit, one]);
    }

    #[test]
    fn free_from_frees_vertices_and_reports_fanout_next_warning_condition() {
        let mut graph = IteGraph::new();
        let lit = graph.add_vertex(IteVertex {
            fanout: Some(FanoutIte { next_present: true }),
            ..IteVertex::c_value(0, 2)
        });
        let root = graph.add_vertex(IteVertex::c_value(0, 3).with_children(lit, lit, lit));

        let report = graph.free_from(root).unwrap();

        assert_eq!(
            report,
            FreeReport {
                vertices: 2,
                fanouts: 1,
                had_non_nil_fanout_next: true,
            }
        );
        assert!(graph.vertex(root).unwrap().freed);
        assert!(graph.vertex(lit).unwrap().freed);
    }

    #[test]
    fn cost_struct_replace_moves_slot_and_rebuilds_existing_match() {
        let mut n1 = SisNode::new(1, "old");
        n1.cost = Some(ActIteCostStruct {
            match_info: Some(ActMatch::default()),
            ..ActIteCostStruct::default()
        });
        let mut n2 = SisNode::new(2, "new");
        n2.cost = Some(ActIteCostStruct {
            match_info: Some(ActMatch {
                a0: Some(NodeId(99)),
                ..ActMatch::default()
            }),
            ..ActIteCostStruct::default()
        });

        act_ite_cost_struct_replace_with_matcher(&mut n1, &mut n2, 7, |node, map_alg| {
            assert_eq!(node, NodeId(1));
            assert_eq!(map_alg, 7);
            Some(ActMatch {
                s0: Some(NodeId(42)),
                ..ActMatch::default()
            })
        })
        .unwrap();

        assert!(n2.cost.is_none());
        assert_eq!(
            n1.cost.as_ref().unwrap().match_info.as_ref().unwrap().s0,
            Some(NodeId(42))
        );
    }

    #[test]
    fn remap_put_node_names_updates_literal_fanins_by_name_and_table() {
        let old_a = SisNode::new(10, "a");
        let new_a = SisNode::new(20, "a_mapped");
        let old_network = network_with(&[old_a]);
        let new_network = network_with(&[new_a]);
        let mut remap = HashMap::new();
        remap.insert(NodeId(10), NodeId(20));

        let mut graph = IteGraph::new();
        let lit = graph.add_vertex(IteVertex {
            fanin: Some(NodeId(10)),
            ..IteVertex::c_value(0, 2)
        });

        act_ite_remap_put_node_names_in_ite(&mut graph, lit, &remap, &old_network, &new_network)
            .unwrap();

        let vertex = graph.vertex(lit).unwrap();
        assert_eq!(vertex.fanin, Some(NodeId(20)));
        assert_eq!(vertex.name.as_deref(), Some("a_mapped"));
    }

    #[test]
    fn remap_update_skips_nodes_with_existing_match() {
        let mut node = SisNode::new(1, "n");
        node.cost = Some(ActIteCostStruct {
            match_info: Some(ActMatch::default()),
            ..ActIteCostStruct::default()
        });

        act_ite_remap_update_ite_fields(
            &HashMap::new(),
            &SisNetwork::new(),
            &SisNetwork::new(),
            &mut node,
        )
        .unwrap();

        assert_eq!(node.cost.unwrap().node, Some(NodeId(1)));
    }

    #[test]
    fn get_node_literal_requires_literal_vertex_and_positive_phase() {
        let mut graph = IteGraph::new();
        let lit = graph.add_vertex(IteVertex {
            fanin: Some(NodeId(3)),
            ..IteVertex::c_value(0, 2)
        });
        let terminal = graph.add_vertex(IteVertex::c_value(0, 0));

        assert_eq!(
            ite_get_node_literal_of_vertex(&graph, lit).unwrap(),
            NodeLiteral {
                node: NodeId(3),
                phase: true,
            }
        );
        assert!(matches!(
            ite_get_node_literal_of_vertex(&graph, terminal),
            Err(IteUtilError::ExpectedLiteral { .. })
        ));
    }

    #[test]
    fn free_nodes_in_multiple_fo_ite_marks_once_and_frees_nonterminal_nodes() {
        let mut network = network_with(&[SisNode::new(1, "n1"), SisNode::new(2, "n2")]);
        let mut graph = IteGraph::new();
        let lit = graph.add_vertex(IteVertex {
            node: Some(NodeId(1)),
            multiple_fo: true,
            ..IteVertex::c_value(0, 2)
        });
        let root = graph.add_vertex(IteVertex {
            node: Some(NodeId(2)),
            multiple_fo: true,
            ..IteVertex::c_value(0, 3).with_children(lit, lit, lit)
        });

        assert_eq!(
            graph
                .free_nodes_in_multiple_fo_ite(root, &mut network)
                .unwrap(),
            2
        );
        assert!(network.node(NodeId(1)).unwrap().freed);
        assert!(network.node(NodeId(2)).unwrap().freed);
        assert_eq!(
            graph
                .free_nodes_in_multiple_fo_ite(root, &mut network)
                .unwrap(),
            0
        );
    }

    #[test]
    fn free_nodes_in_multiple_fo_act_recurses_low_high() {
        let mut network = network_with(&[SisNode::new(1, "n1"), SisNode::new(2, "n2")]);
        let mut graph = ActGraph::new();
        let low = graph.add_vertex(ActVertex {
            node: Some(NodeId(1)),
            multiple_fo: true,
            ..ActVertex::new(0, 2)
        });
        let high = graph.add_vertex(ActVertex::new(0, 1));
        let root = graph.add_vertex(ActVertex {
            node: Some(NodeId(2)),
            multiple_fo: true,
            ..ActVertex::new(0, 3).with_children(low, high)
        });

        assert_eq!(
            graph
                .free_nodes_in_multiple_fo_act(root, &mut network)
                .unwrap(),
            2
        );
        assert!(network.node(NodeId(1)).unwrap().freed);
        assert!(network.node(NodeId(2)).unwrap().freed);
    }

    #[test]
    fn free_ite_network_clears_each_node_cost_owned_fields() {
        let mut graph = IteGraph::new();
        let root = graph.add_vertex(IteVertex::c_value(0, 2));
        let mut node = SisNode::new(1, "n");
        node.cost = Some(ActIteCostStruct {
            ite: Some(graph),
            ite_root: Some(root),
            match_info: Some(ActMatch::default()),
            network: Some(network_with(&[SisNode::new(2, "inner")])),
            ..ActIteCostStruct::default()
        });
        let mut network = network_with(&[node]);

        assert_eq!(act_free_ite_network(&mut network).unwrap(), 1);
        let cost = network.node(NodeId(1)).unwrap().cost.as_ref().unwrap();
        assert!(cost.ite.is_none());
        assert!(cost.match_info.is_none());
        assert!(cost.network.is_none());
    }
}
