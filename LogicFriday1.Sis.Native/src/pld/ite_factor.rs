//! Owned-data Rust port for factored-form ITE construction and selection.
//!
//! The original SIS routine builds a temporary network from one node, decomposes
//! that network, creates ITEs for its DFS nodes, then keeps the mapped result
//! only when it improves the node cost. This module models that control flow
//! without legacy per-file C ABI entry points.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IteFactorError {
    MissingNode(NodeId),
    MissingVertex(IteVertexId),
    MissingFanin { node: NodeId },
    MissingIte { node: NodeId },
    ExpectedPrimaryOutput { node: NodeId, kind: NodeKind },
    ExpectedPositiveLiteral { vertex: IteVertexId },
    UnknownFaninName(String),
    LiteralNameNotInOriginalFanins(String),
}

impl fmt::Display for IteFactorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(f, "missing node {}", node.0),
            Self::MissingVertex(vertex) => write!(f, "missing ITE vertex {}", vertex.0),
            Self::MissingFanin { node } => write!(f, "node {} has no fanin", node.0),
            Self::MissingIte { node } => write!(f, "node {} has no ITE", node.0),
            Self::ExpectedPrimaryOutput { node, kind } => {
                write!(f, "node {} is {kind:?}, expected primary output", node.0)
            }
            Self::ExpectedPositiveLiteral { vertex } => {
                write!(f, "ITE vertex {} is not a positive literal", vertex.0)
            }
            Self::UnknownFaninName(name) => write!(f, "temporary fanin {name} is not in network"),
            Self::LiteralNameNotInOriginalFanins(name) => {
                write!(f, "temporary fanin {name} is not an original node fanin")
            }
        }
    }
}

impl Error for IteFactorError {}

pub type IteFactorResult<T> = Result<T, IteFactorError>;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FactorNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub fanin_names: HashMap<NodeId, String>,
    pub literal_count: usize,
    pub factor_literal_count: usize,
    pub ite_root: Option<IteVertexId>,
    pub cost: Option<ActIteCost>,
}

impl FactorNode {
    pub fn new(id: usize, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanin_names: HashMap::new(),
            literal_count: 0,
            factor_literal_count: 0,
            ite_root: None,
            cost: None,
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = NodeId>) -> Self {
        self.fanins = fanins.into_iter().collect();
        self
    }

    pub fn with_named_fanins(
        mut self,
        fanins: impl IntoIterator<Item = (NodeId, impl Into<String>)>,
    ) -> Self {
        self.fanins.clear();
        self.fanin_names.clear();
        for (fanin, name) in fanins {
            self.fanins.push(fanin);
            self.fanin_names.insert(fanin, name.into());
        }
        self
    }

    pub fn with_literal_counts(
        mut self,
        literal_count: usize,
        factor_literal_count: usize,
    ) -> Self {
        self.literal_count = literal_count;
        self.factor_literal_count = factor_literal_count;
        self
    }

    pub fn with_cost(mut self, cost: i32) -> Self {
        self.cost = Some(ActIteCost {
            cost,
            ..ActIteCost::default()
        });
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FactorNetwork {
    nodes: HashMap<NodeId, FactorNode>,
    names: HashMap<String, NodeId>,
    dfs_order: Vec<NodeId>,
    freed: bool,
}

impl FactorNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: FactorNode) {
        self.names.insert(node.name.clone(), node.id);
        self.dfs_order.push(node.id);
        self.nodes.insert(node.id, node);
    }

    pub fn node(&self, id: NodeId) -> IteFactorResult<&FactorNode> {
        self.nodes.get(&id).ok_or(IteFactorError::MissingNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> IteFactorResult<&mut FactorNode> {
        self.nodes
            .get_mut(&id)
            .ok_or(IteFactorError::MissingNode(id))
    }

    pub fn find_node(&self, name: &str) -> Option<NodeId> {
        self.names.get(name).copied()
    }

    pub fn node_name(&self, id: NodeId) -> IteFactorResult<&str> {
        Ok(self.node(id)?.name.as_str())
    }

    pub fn set_ite_root(&mut self, id: NodeId, root: IteVertexId) -> IteFactorResult<()> {
        self.node_mut(id)?.ite_root = Some(root);
        Ok(())
    }

    pub fn dfs_order(&self) -> &[NodeId] {
        &self.dfs_order
    }

    pub fn last_dfs_node(&self) -> Option<NodeId> {
        self.dfs_order.last().copied()
    }

    pub fn mark_freed(&mut self) {
        self.freed = true;
    }

    pub fn was_freed(&self) -> bool {
        self.freed
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActIteCost {
    pub cost: i32,
    pub arrival_time: f64,
    pub ite_root: Option<IteVertexId>,
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

#[derive(Clone, Debug, PartialEq)]
pub struct IteVertex {
    pub id: IteVertexId,
    pub value: IteValue,
    pub phase: bool,
    pub if_child: Option<IteVertexId>,
    pub then_child: Option<IteVertexId>,
    pub else_child: Option<IteVertexId>,
    pub fanin: Option<NodeId>,
    pub name: Option<String>,
    pub arrival_time: f64,
}

impl IteVertex {
    pub fn terminal(id: usize, value: bool) -> Self {
        Self::new(id, if value { IteValue::One } else { IteValue::Zero })
    }

    pub fn literal(id: usize, fanin: NodeId, name: impl Into<String>) -> Self {
        Self {
            fanin: Some(fanin),
            name: Some(name.into()),
            ..Self::new(id, IteValue::Literal)
        }
    }

    pub fn ite(
        id: usize,
        if_child: IteVertexId,
        then_child: IteVertexId,
        else_child: IteVertexId,
    ) -> Self {
        Self {
            if_child: Some(if_child),
            then_child: Some(then_child),
            else_child: Some(else_child),
            ..Self::new(id, IteValue::IfThenElse)
        }
    }

    fn new(id: usize, value: IteValue) -> Self {
        Self {
            id: IteVertexId(id),
            value,
            phase: true,
            if_child: None,
            then_child: None,
            else_child: None,
            fanin: None,
            name: None,
            arrival_time: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
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

    pub fn vertex(&self, id: IteVertexId) -> IteFactorResult<&IteVertex> {
        self.vertices
            .get(id.0)
            .ok_or(IteFactorError::MissingVertex(id))
    }

    pub fn vertex_mut(&mut self, id: IteVertexId) -> IteFactorResult<&mut IteVertex> {
        self.vertices
            .get_mut(id.0)
            .ok_or(IteFactorError::MissingVertex(id))
    }

    pub fn child_ids(&self, id: IteVertexId) -> IteFactorResult<[Option<IteVertexId>; 3]> {
        let vertex = self.vertex(id)?;
        Ok([vertex.if_child, vertex.then_child, vertex.else_child])
    }
}

pub trait IteFactorHooks<Init> {
    fn create_network_from_node(&mut self, node: &FactorNode) -> IteFactorResult<FactorNetwork>;
    fn decomp_good_network(&mut self, network: &mut FactorNetwork) -> IteFactorResult<()>;
    fn make_intermediate_ite(
        &mut self,
        node: NodeId,
        init_param: &Init,
        network: &mut FactorNetwork,
        graph: &mut IteGraph,
    ) -> IteFactorResult<()>;
    fn modify_fields_node(
        &mut self,
        node: NodeId,
        network: &mut FactorNetwork,
        graph: &mut IteGraph,
    ) -> IteFactorResult<()>;
    fn make_tree_and_map(
        &mut self,
        node: &mut FactorNode,
        graph: &mut IteGraph,
        root: IteVertexId,
    ) -> IteFactorResult<i32>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NativeIteFactorHooks;

impl<Init> IteFactorHooks<Init> for NativeIteFactorHooks {
    fn create_network_from_node(&mut self, node: &FactorNode) -> IteFactorResult<FactorNetwork> {
        let mut network = FactorNetwork::new();
        let next_id = node
            .fanins
            .iter()
            .map(|fanin| fanin.0)
            .max()
            .unwrap_or(node.id.0)
            .max(node.id.0)
            + 1;
        let internal = NodeId(next_id);
        let output = NodeId(next_id + 1);

        for fanin in &node.fanins {
            let name = node
                .fanin_names
                .get(fanin)
                .cloned()
                .unwrap_or_else(|| format!("n{}", fanin.0));
            network.add_node(FactorNode::new(fanin.0, name, NodeKind::PrimaryInput));
        }

        network.add_node(
            FactorNode::new(internal.0, node.name.clone(), NodeKind::Internal)
                .with_fanins(node.fanins.iter().copied())
                .with_literal_counts(node.literal_count, node.factor_literal_count),
        );
        network.add_node(
            FactorNode::new(output.0, format!("{}_out", node.name), NodeKind::PrimaryOutput)
                .with_fanins([internal]),
        );
        Ok(network)
    }

    fn decomp_good_network(&mut self, _network: &mut FactorNetwork) -> IteFactorResult<()> {
        Ok(())
    }

    fn make_intermediate_ite(
        &mut self,
        node: NodeId,
        _init_param: &Init,
        network: &mut FactorNetwork,
        graph: &mut IteGraph,
    ) -> IteFactorResult<()> {
        match network.node(node)?.kind {
            NodeKind::PrimaryInput => {
                let name = network.node_name(node)?.to_owned();
                let root = graph.add_vertex(IteVertex::literal(0, node, name));
                network.set_ite_root(node, root)
            }
            NodeKind::Internal => {
                let fanins = network.node(node)?.fanins.clone();
                let root = build_native_factored_ite(graph, &fanins, 0)?;
                network.set_ite_root(node, root)
            }
            NodeKind::PrimaryOutput => Ok(()),
        }
    }

    fn modify_fields_node(
        &mut self,
        _node: NodeId,
        _network: &mut FactorNetwork,
        _graph: &mut IteGraph,
    ) -> IteFactorResult<()> {
        Ok(())
    }

    fn make_tree_and_map(
        &mut self,
        _node: &mut FactorNode,
        graph: &mut IteGraph,
        root: IteVertexId,
    ) -> IteFactorResult<i32> {
        Ok(get_value_2_vertices(graph, root)?.len() as i32)
    }
}

pub fn create_from_factored_form_native<Init>(
    node: &FactorNode,
    init_param: &Init,
) -> IteFactorResult<Option<(IteGraph, IteVertexId)>> {
    let mut hooks = NativeIteFactorHooks;
    create_from_factored_form(node, init_param, &mut hooks)
}

pub fn map_factored_form_native<Init>(
    node: &mut FactorNode,
    init_param: &Init,
) -> IteFactorResult<i32> {
    let mut hooks = NativeIteFactorHooks;
    let mut graph = IteGraph::new();
    map_factored_form(node, init_param, &mut graph, &mut hooks)
}

pub fn create_from_factored_form<Init, Hooks>(
    node: &FactorNode,
    init_param: &Init,
    hooks: &mut Hooks,
) -> IteFactorResult<Option<(IteGraph, IteVertexId)>>
where
    Hooks: IteFactorHooks<Init>,
{
    if node.literal_count == node.factor_literal_count {
        return Ok(None);
    }

    let mut graph = IteGraph::new();
    let mut network = hooks.create_network_from_node(node)?;
    hooks.decomp_good_network(&mut network)?;

    for network_node in network.dfs_order().to_vec() {
        hooks.make_intermediate_ite(network_node, init_param, &mut network, &mut graph)?;
        hooks.modify_fields_node(network_node, &mut network, &mut graph)?;
    }

    let output = network
        .last_dfs_node()
        .ok_or(IteFactorError::MissingFanin { node: node.id })?;
    let output_node = network.node(output)?;
    if output_node.kind != NodeKind::PrimaryOutput {
        return Err(IteFactorError::ExpectedPrimaryOutput {
            node: output,
            kind: output_node.kind,
        });
    }

    let driver = *output_node
        .fanins
        .first()
        .ok_or(IteFactorError::MissingFanin { node: output })?;
    let root = network
        .node(driver)?
        .ite_root
        .ok_or(IteFactorError::MissingIte { node: driver })?;

    correct_primary_inputs(&mut graph, root, node, Some(&network))?;
    network.mark_freed();
    Ok(Some((graph, root)))
}

pub fn map_factored_form<Init, Hooks>(
    node: &mut FactorNode,
    init_param: &Init,
    graph: &mut IteGraph,
    hooks: &mut Hooks,
) -> IteFactorResult<i32>
where
    Hooks: IteFactorHooks<Init>,
{
    if matches!(node.kind, NodeKind::PrimaryInput | NodeKind::PrimaryOutput) {
        return Ok(0);
    }

    let original_cost = node.cost.clone().unwrap_or_default();
    let (mut candidate_graph, root) = match create_from_factored_form(node, init_param, hooks)? {
        Some(candidate) => candidate,
        None => return Ok(0),
    };

    let mut candidate_node = node.clone();
    candidate_node.cost = Some(ActIteCost {
        ite_root: Some(root),
        ..ActIteCost::default()
    });
    let mapped_cost = hooks.make_tree_and_map(&mut candidate_node, &mut candidate_graph, root)?;
    let arrival_time = candidate_graph.vertex(root)?.arrival_time;
    let gain = original_cost.cost - mapped_cost;

    if gain > 0 {
        *graph = candidate_graph;
        node.cost = Some(ActIteCost {
            cost: mapped_cost,
            arrival_time,
            ite_root: Some(root),
        });
        return Ok(gain);
    }

    node.cost = Some(original_cost);
    Ok(0)
}

fn build_native_factored_ite(
    graph: &mut IteGraph,
    fanins: &[NodeId],
    index: usize,
) -> IteFactorResult<IteVertexId> {
    if index >= fanins.len() {
        return Ok(graph.add_vertex(IteVertex::terminal(0, true)));
    }

    let fanin = fanins[index];
    let literal = graph.add_vertex(IteVertex::literal(0, fanin, format!("n{}", fanin.0)));
    if index + 1 == fanins.len() {
        return Ok(literal);
    }

    let then_child = build_native_factored_ite(graph, fanins, index + 1)?;
    let else_child = graph.add_vertex(IteVertex::terminal(0, false));
    Ok(graph.add_vertex(IteVertex::ite(0, literal, then_child, else_child)))
}

pub fn correct_primary_inputs(
    graph: &mut IteGraph,
    root: IteVertexId,
    original_node: &FactorNode,
    temporary_network: Option<&FactorNetwork>,
) -> IteFactorResult<usize> {
    let literal_vertices = get_value_2_vertices(graph, root)?;
    let mut corrected = 0;

    for vertex_id in literal_vertices {
        let current_fanin = graph
            .vertex(vertex_id)?
            .fanin
            .ok_or(IteFactorError::ExpectedPositiveLiteral { vertex: vertex_id })?;
        let temporary_name = match temporary_network {
            Some(network) => network.node_name(current_fanin)?.to_owned(),
            None => graph
                .vertex(vertex_id)?
                .name
                .clone()
                .ok_or(IteFactorError::ExpectedPositiveLiteral { vertex: vertex_id })?,
        };

        let original_fanin = if let Some(network) = temporary_network {
            let temporary_node = network
                .find_node(&temporary_name)
                .ok_or_else(|| IteFactorError::UnknownFaninName(temporary_name.clone()))?;
            let name = network.node_name(temporary_node)?;
            find_original_fanin_by_name(original_node, name)?
        } else {
            find_original_fanin_by_name(original_node, &temporary_name)?
        };

        let vertex = graph.vertex_mut(vertex_id)?;
        vertex.fanin = Some(original_fanin);
        vertex.name = Some(temporary_name);
        corrected += 1;
    }

    Ok(corrected)
}

pub fn get_value_2_vertices(
    graph: &IteGraph,
    root: IteVertexId,
) -> IteFactorResult<Vec<IteVertexId>> {
    let mut seen = HashSet::new();
    let mut vertices = Vec::new();
    collect_value_2_vertices(graph, root, &mut seen, &mut vertices)?;
    Ok(vertices)
}

fn collect_value_2_vertices(
    graph: &IteGraph,
    root: IteVertexId,
    seen: &mut HashSet<IteVertexId>,
    vertices: &mut Vec<IteVertexId>,
) -> IteFactorResult<()> {
    if !seen.insert(root) {
        return Ok(());
    }

    let vertex = graph.vertex(root)?;
    match vertex.value {
        IteValue::Zero | IteValue::One => Ok(()),
        IteValue::Literal => {
            if !vertex.phase {
                return Err(IteFactorError::ExpectedPositiveLiteral { vertex: root });
            }
            vertices.push(root);
            Ok(())
        }
        IteValue::IfThenElse => {
            for child in graph.child_ids(root)?.into_iter().flatten() {
                collect_value_2_vertices(graph, child, seen, vertices)?;
            }
            Ok(())
        }
    }
}

fn find_original_fanin_by_name(node: &FactorNode, name: &str) -> IteFactorResult<NodeId> {
    node.fanins
        .iter()
        .copied()
        .find(|fanin| {
            node.fanin_names
                .get(fanin)
                .is_some_and(|fanin_name| fanin_name == name)
                || fanin.0.to_string() == name
                || format!("n{}", fanin.0) == name
        })
        .ok_or_else(|| IteFactorError::LiteralNameNotInOriginalFanins(name.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingHooks {
        network: FactorNetwork,
        call_order: Vec<NodeId>,
        mapped_cost: i32,
        root_arrival_time: f64,
    }

    impl IteFactorHooks<()> for RecordingHooks {
        fn create_network_from_node(
            &mut self,
            _node: &FactorNode,
        ) -> IteFactorResult<FactorNetwork> {
            Ok(self.network.clone())
        }

        fn decomp_good_network(&mut self, _network: &mut FactorNetwork) -> IteFactorResult<()> {
            Ok(())
        }

        fn make_intermediate_ite(
            &mut self,
            node: NodeId,
            _init_param: &(),
            network: &mut FactorNetwork,
            graph: &mut IteGraph,
        ) -> IteFactorResult<()> {
            self.call_order.push(node);
            match network.node(node)?.kind {
                NodeKind::PrimaryInput => {
                    let name = network.node_name(node)?.to_owned();
                    let root = graph.add_vertex(IteVertex::literal(0, node, name));
                    network.set_ite_root(node, root)?;
                }
                NodeKind::Internal => {
                    let fanin = network.node(node)?.fanins[0];
                    let name = network.node_name(fanin)?.to_owned();
                    let literal = graph.add_vertex(IteVertex::literal(0, fanin, name));
                    graph.vertex_mut(literal)?.arrival_time = self.root_arrival_time;
                    network.set_ite_root(node, literal)?;
                }
                NodeKind::PrimaryOutput => {}
            }
            Ok(())
        }

        fn modify_fields_node(
            &mut self,
            _node: NodeId,
            _network: &mut FactorNetwork,
            _graph: &mut IteGraph,
        ) -> IteFactorResult<()> {
            Ok(())
        }

        fn make_tree_and_map(
            &mut self,
            _node: &mut FactorNode,
            _graph: &mut IteGraph,
            _root: IteVertexId,
        ) -> IteFactorResult<i32> {
            Ok(self.mapped_cost)
        }
    }

    fn original_node() -> FactorNode {
        FactorNode::new(100, "f", NodeKind::Internal)
            .with_named_fanins([(NodeId(1), "a"), (NodeId(2), "b")])
            .with_literal_counts(6, 3)
            .with_cost(5)
    }

    fn temporary_network() -> FactorNetwork {
        let mut network = FactorNetwork::new();
        network.add_node(FactorNode::new(10, "a", NodeKind::PrimaryInput));
        network.add_node(FactorNode::new(11, "b", NodeKind::PrimaryInput));
        network.add_node(FactorNode::new(12, "mid", NodeKind::Internal).with_fanins([NodeId(10)]));
        network.add_node(
            FactorNode::new(13, "out", NodeKind::PrimaryOutput).with_fanins([NodeId(12)]),
        );
        network
    }

    #[test]
    fn equal_literal_counts_skip_factored_form() {
        let node = FactorNode::new(1, "f", NodeKind::Internal).with_literal_counts(3, 3);
        let mut hooks = RecordingHooks::default();

        assert_eq!(
            create_from_factored_form(&node, &(), &mut hooks).unwrap(),
            None
        );
        assert!(hooks.call_order.is_empty());
    }

    #[test]
    fn create_from_factored_form_builds_dfs_nodes_and_corrects_pi_fanins() {
        let node = original_node();
        let mut hooks = RecordingHooks {
            network: temporary_network(),
            ..RecordingHooks::default()
        };

        let (graph, root) = create_from_factored_form(&node, &(), &mut hooks)
            .unwrap()
            .unwrap();

        assert_eq!(
            hooks.call_order,
            vec![NodeId(10), NodeId(11), NodeId(12), NodeId(13)]
        );
        let root_vertex = graph.vertex(root).unwrap();
        assert_eq!(root_vertex.fanin, Some(NodeId(1)));
        assert_eq!(root_vertex.name.as_deref(), Some("a"));
    }

    #[test]
    fn get_value_2_vertices_de_duplicates_shared_literals() {
        let mut graph = IteGraph::new();
        let literal = graph.add_vertex(IteVertex::literal(0, NodeId(1), "n1"));
        let one = graph.add_vertex(IteVertex::terminal(0, true));
        let root = graph.add_vertex(IteVertex::ite(0, literal, one, literal));

        assert_eq!(get_value_2_vertices(&graph, root).unwrap(), vec![literal]);
    }

    #[test]
    fn negative_literal_is_rejected_like_the_c_assertion() {
        let mut graph = IteGraph::new();
        let literal = graph.add_vertex(IteVertex {
            phase: false,
            ..IteVertex::literal(0, NodeId(1), "n1")
        });

        assert!(matches!(
            get_value_2_vertices(&graph, literal),
            Err(IteFactorError::ExpectedPositiveLiteral { .. })
        ));
    }

    #[test]
    fn map_factored_form_accepts_only_strict_cost_improvement() {
        let mut node = original_node();
        let mut hooks = RecordingHooks {
            network: temporary_network(),
            mapped_cost: 3,
            root_arrival_time: 2.5,
            ..RecordingHooks::default()
        };
        let mut graph = IteGraph::new();

        assert_eq!(
            map_factored_form(&mut node, &(), &mut graph, &mut hooks).unwrap(),
            2
        );

        assert_eq!(node.cost.as_ref().unwrap().cost, 3);
        assert_eq!(node.cost.as_ref().unwrap().arrival_time, 2.5);
        assert!(node.cost.as_ref().unwrap().ite_root.is_some());
    }

    #[test]
    fn map_factored_form_restores_original_cost_when_gain_is_not_positive() {
        let mut node = original_node();
        let original = node.cost.clone();
        let mut hooks = RecordingHooks {
            network: temporary_network(),
            mapped_cost: 5,
            ..RecordingHooks::default()
        };
        let mut graph = IteGraph::new();

        assert_eq!(
            map_factored_form(&mut node, &(), &mut graph, &mut hooks).unwrap(),
            0
        );

        assert_eq!(node.cost, original);
    }

    #[test]
    fn primary_input_and_output_nodes_are_not_mapped() {
        for kind in [NodeKind::PrimaryInput, NodeKind::PrimaryOutput] {
            let mut node = FactorNode::new(1, "p", kind).with_cost(7);
            let mut hooks = RecordingHooks::default();
            let mut graph = IteGraph::new();

            assert_eq!(
                map_factored_form(&mut node, &(), &mut graph, &mut hooks).unwrap(),
                0
            );
            assert_eq!(node.cost.as_ref().unwrap().cost, 7);
        }
    }

    #[test]
    fn native_entry_points_build_and_map_owned_factored_form() {
        let mut node = original_node();

        let (graph, root) = create_from_factored_form_native(&node, &())
            .unwrap()
            .unwrap();

        assert_eq!(graph.vertex(root).unwrap().value, IteValue::IfThenElse);
        assert_eq!(map_factored_form_native(&mut node, &()).unwrap(), 3);
        assert_eq!(node.cost.as_ref().unwrap().cost, 2);
    }

    #[test]
    fn no_legacy_c_abi_or_dependency_metadata_tokens_are_present() {
        let source = include_str!("ite_factor.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("be", "ad")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
