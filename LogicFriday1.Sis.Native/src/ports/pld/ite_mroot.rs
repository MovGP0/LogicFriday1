//! Native Rust model for `LogicSynthesis/sis/pld/ite_mroot.c`.
//!
//! The C file orchestrates a multiple-root ITE mapping pass over a SIS network:
//! it builds ITEs for DFS-ordered nodes, rewrites buffer leaves to point at
//! already-built fanin ITEs, initializes mapper metadata across all primary
//! output roots, maps each root in reverse order, and finally frees the shared
//! ITE DAG. This module ports those graph transformations onto owned Rust data.
//! SIS-backed node/network traversal, ITE construction, and mapper integration
//! remain explicit dependency errors until their native ports are available.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IteMrootError {
    MissingNode(NodeId),
    MissingIteForNode(NodeId),
    MissingVertex(IteVertexId),
    MissingChild {
        vertex: IteVertexId,
        child: IteChild,
    },
    MissingFanin {
        vertex: IteVertexId,
    },
    PrimaryOutputWithoutFanin(NodeId),
    ExpectedPositiveLiteral {
        vertex: IteVertexId,
    },
    ExpectedIfThenElse {
        vertex: IteVertexId,
        value: IteValue,
    },
}

impl fmt::Display for IteMrootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(f, "missing SIS node {}", node.0),
            Self::MissingIteForNode(node) => write!(f, "node {} has no ITE root", node.0),
            Self::MissingVertex(vertex) => write!(f, "missing ITE vertex {}", vertex.0),
            Self::MissingChild { vertex, child } => {
                write!(f, "ITE vertex {} has no {child:?} child", vertex.0)
            }
            Self::MissingFanin { vertex } => {
                write!(f, "ITE literal vertex {} has no fanin node", vertex.0)
            }
            Self::PrimaryOutputWithoutFanin(node) => {
                write!(f, "primary output node {} has no fanin", node.0)
            }
            Self::ExpectedPositiveLiteral { vertex } => {
                write!(f, "ITE vertex {} is not a positive literal", vertex.0)
            }
            Self::ExpectedIfThenElse { vertex, value } => write!(
                f,
                "ITE vertex {} has value {value:?}, expected an if-then-else vertex",
                vertex.0
            ),
        }
    }
}

impl Error for IteMrootError {}

pub type IteMrootResult<T> = Result<T, IteMrootError>;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SisNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub ite_root: Option<IteVertexId>,
}

impl SisNode {
    pub fn new(id: usize, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind,
            fanins: Vec::new(),
            ite_root: None,
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = NodeId>) -> Self {
        self.fanins = fanins.into_iter().collect();
        self
    }

    pub fn with_ite_root(mut self, root: IteVertexId) -> Self {
        self.ite_root = Some(root);
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SisNetwork {
    nodes: HashMap<NodeId, SisNode>,
    dfs_order: Vec<NodeId>,
    primary_outputs: Vec<NodeId>,
}

impl SisNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: SisNode) {
        if node.kind == NodeKind::PrimaryOutput && !self.primary_outputs.contains(&node.id) {
            self.primary_outputs.push(node.id);
        }
        self.dfs_order.push(node.id);
        self.nodes.insert(node.id, node);
    }

    pub fn node(&self, id: NodeId) -> IteMrootResult<&SisNode> {
        self.nodes.get(&id).ok_or(IteMrootError::MissingNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> IteMrootResult<&mut SisNode> {
        self.nodes
            .get_mut(&id)
            .ok_or(IteMrootError::MissingNode(id))
    }

    pub fn node_kind(&self, id: NodeId) -> IteMrootResult<NodeKind> {
        Ok(self.node(id)?.kind)
    }

    pub fn set_ite_root(&mut self, id: NodeId, root: IteVertexId) -> IteMrootResult<()> {
        self.node_mut(id)?.ite_root = Some(root);
        Ok(())
    }

    pub fn ite_root(&self, id: NodeId) -> IteMrootResult<IteVertexId> {
        self.node(id)?
            .ite_root
            .ok_or(IteMrootError::MissingIteForNode(id))
    }

    pub fn dfs_order(&self) -> &[NodeId] {
        &self.dfs_order
    }

    pub fn primary_outputs(&self) -> &[NodeId] {
        &self.primary_outputs
    }

    pub fn primary_output_roots(&self) -> IteMrootResult<Vec<IteVertexId>> {
        let mut roots = Vec::new();
        for po in &self.primary_outputs {
            let fanin = *self
                .node(*po)?
                .fanins
                .first()
                .ok_or(IteMrootError::PrimaryOutputWithoutFanin(*po))?;
            if self.node_kind(fanin)? != NodeKind::PrimaryInput {
                roots.push(self.ite_root(fanin)?);
            }
        }
        Ok(roots)
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IteChild {
    If,
    Then,
    Else,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IteVertex {
    pub id: IteVertexId,
    pub value: IteValue,
    pub if_child: Option<IteVertexId>,
    pub then_child: Option<IteVertexId>,
    pub else_child: Option<IteVertexId>,
    pub fanin: Option<NodeId>,
    pub phase: bool,
    pub multiple_fo: usize,
    pub multiple_fo_for_mapping: usize,
    pub cost: i32,
    pub arrival_time: f64,
    pub pattern_num: i32,
    pub mapped: bool,
    pub mark: i32,
    pub freed: bool,
}

impl IteVertex {
    pub fn terminal(id: usize, value: bool) -> Self {
        Self::new(id, if value { IteValue::One } else { IteValue::Zero })
    }

    pub fn literal(id: usize, fanin: NodeId, phase: bool) -> Self {
        Self {
            fanin: Some(fanin),
            phase,
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
            if_child: None,
            then_child: None,
            else_child: None,
            fanin: None,
            phase: true,
            multiple_fo: 0,
            multiple_fo_for_mapping: 0,
            cost: 0,
            arrival_time: 0.0,
            pattern_num: -1,
            mapped: false,
            mark: 0,
            freed: false,
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

    pub fn vertex(&self, id: IteVertexId) -> IteMrootResult<&IteVertex> {
        self.vertices
            .get(id.0)
            .ok_or(IteMrootError::MissingVertex(id))
    }

    pub fn vertex_mut(&mut self, id: IteVertexId) -> IteMrootResult<&mut IteVertex> {
        self.vertices
            .get_mut(id.0)
            .ok_or(IteMrootError::MissingVertex(id))
    }

    pub fn child(&self, id: IteVertexId, child: IteChild) -> IteMrootResult<IteVertexId> {
        let vertex = self.vertex(id)?;
        let child_id = match child {
            IteChild::If => vertex.if_child,
            IteChild::Then => vertex.then_child,
            IteChild::Else => vertex.else_child,
        };
        child_id.ok_or(IteMrootError::MissingChild { vertex: id, child })
    }

    pub fn set_child(
        &mut self,
        id: IteVertexId,
        child: IteChild,
        value: IteVertexId,
    ) -> IteMrootResult<()> {
        let vertex = self.vertex_mut(id)?;
        match child {
            IteChild::If => vertex.if_child = Some(value),
            IteChild::Then => vertex.then_child = Some(value),
            IteChild::Else => vertex.else_child = Some(value),
        }
        Ok(())
    }

    pub fn is_buffer(&self, id: IteVertexId) -> IteMrootResult<bool> {
        let vertex = self.vertex(id)?;
        if vertex.value != IteValue::IfThenElse {
            return Ok(false);
        }

        let if_vertex = self.vertex(self.child(id, IteChild::If)?)?;
        let then_vertex = self.vertex(self.child(id, IteChild::Then)?)?;
        let else_vertex = self.vertex(self.child(id, IteChild::Else)?)?;

        Ok(if_vertex.value == IteValue::Literal
            && if_vertex.phase
            && then_vertex.value == IteValue::One
            && else_vertex.value == IteValue::Zero)
    }

    pub fn positive_literal_fanin(&self, id: IteVertexId) -> IteMrootResult<NodeId> {
        let vertex = self.vertex(id)?;
        if vertex.value != IteValue::Literal || !vertex.phase {
            return Err(IteMrootError::ExpectedPositiveLiteral { vertex: id });
        }
        vertex
            .fanin
            .ok_or(IteMrootError::MissingFanin { vertex: id })
    }

    fn reset_mapping_fields(&mut self, id: IteVertexId, multiple_fo: usize) -> IteMrootResult<()> {
        let vertex = self.vertex_mut(id)?;
        vertex.pattern_num = -1;
        vertex.cost = 0;
        vertex.arrival_time = 0.0;
        vertex.mapped = false;
        vertex.multiple_fo = multiple_fo;
        vertex.multiple_fo_for_mapping = 0;
        vertex.mark = 0;
        Ok(())
    }

    fn mark_freed(&mut self, id: IteVertexId) -> IteMrootResult<bool> {
        let vertex = self.vertex_mut(id)?;
        let was_live = !vertex.freed;
        vertex.freed = true;
        Ok(was_live)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModifyReport {
    pub replaced_roots: usize,
    pub replaced_children: usize,
    pub freed_vertices: Vec<IteVertexId>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MappingReport {
    pub roots: Vec<IteVertexId>,
    pub mapped_in_order: Vec<IteVertexId>,
    pub total_mux_structures: i32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FreeReport {
    pub freed_vertices: Vec<IteVertexId>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CreateAndMapReport {
    pub modify: ModifyReport,
    pub mapping: MappingReport,
    pub free: FreeReport,
}

type IteBuilder<'a, Init> =
    dyn FnMut(NodeId, &Init, &SisNetwork, &mut IteGraph) -> IteMrootResult<Option<IteVertexId>> + 'a;

type IteMapper<'a> = dyn FnMut(IteVertexId, &mut IteGraph) -> IteMrootResult<i32> + 'a;

pub fn create_and_map_mroot_network<Init>(
    network: &mut SisNetwork,
    graph: &mut IteGraph,
    init_param: &Init,
    builder: &mut IteBuilder<'_, Init>,
    mapper: &mut IteMapper<'_>,
) -> IteMrootResult<CreateAndMapReport> {
    let modify = create_mroot_ite_network(network, graph, init_param, builder)?;
    let mapping = make_tree_and_map(network, graph, mapper)?;
    let free = traverse_and_free(network, graph)?;

    Ok(CreateAndMapReport {
        modify,
        mapping,
        free,
    })
}

pub fn create_mroot_ite_network<Init>(
    network: &mut SisNetwork,
    graph: &mut IteGraph,
    init_param: &Init,
    builder: &mut IteBuilder<'_, Init>,
) -> IteMrootResult<ModifyReport> {
    create_mroot_ite_network_with_builder(network, graph, init_param, builder)
}

pub fn make_tree_and_map(
    network: &SisNetwork,
    graph: &mut IteGraph,
    mapper: &mut IteMapper<'_>,
) -> IteMrootResult<MappingReport> {
    make_tree_and_map_with_mapper(network, graph, mapper)
}

pub fn create_mroot_ite_network_with_builder<Init, Builder>(
    network: &mut SisNetwork,
    graph: &mut IteGraph,
    init_param: &Init,
    mut builder: Builder,
) -> IteMrootResult<ModifyReport>
where
    Builder:
        FnMut(NodeId, &Init, &SisNetwork, &mut IteGraph) -> IteMrootResult<Option<IteVertexId>>,
{
    let order = network.dfs_order().to_vec();
    let mut report = ModifyReport::default();

    for node in order {
        if let Some(root) = builder(node, init_param, network, graph)? {
            network.set_ite_root(node, root)?;
        }
        let node_report = modify_fields_node(network, graph, node)?;
        report.replaced_roots += node_report.replaced_roots;
        report.replaced_children += node_report.replaced_children;
        report.freed_vertices.extend(node_report.freed_vertices);
    }

    Ok(report)
}

pub fn modify_fields_node(
    network: &mut SisNetwork,
    graph: &mut IteGraph,
    node: NodeId,
) -> IteMrootResult<ModifyReport> {
    if matches!(
        network.node_kind(node)?,
        NodeKind::PrimaryInput | NodeKind::PrimaryOutput
    ) {
        return Ok(ModifyReport::default());
    }

    let root = network.ite_root(node)?;
    if graph.is_buffer(root)? {
        let fanin_literal = graph.child(root, IteChild::If)?;
        let fanin = graph.positive_literal_fanin(fanin_literal)?;
        if network.node_kind(fanin)? != NodeKind::PrimaryInput {
            let replacement = network.ite_root(fanin)?;
            network.set_ite_root(node, replacement)?;
            let mut report = ModifyReport {
                replaced_roots: 1,
                ..ModifyReport::default()
            };
            collect_and_mark_freed(graph, root, &mut report.freed_vertices)?;
            return Ok(report);
        }
        return Ok(ModifyReport::default());
    }

    let mut visited = HashSet::new();
    let mut free_candidates = HashSet::new();
    let mut report = ModifyReport::default();
    modify_fields_ite(
        network,
        graph,
        root,
        &mut visited,
        &mut free_candidates,
        &mut report,
    )?;

    for candidate in free_candidates {
        if !visited.contains(&candidate) && graph.mark_freed(candidate)? {
            report.freed_vertices.push(candidate);
        }
    }
    Ok(report)
}

pub fn modify_fields_ite(
    network: &SisNetwork,
    graph: &mut IteGraph,
    root: IteVertexId,
    visited: &mut HashSet<IteVertexId>,
    free_candidates: &mut HashSet<IteVertexId>,
    report: &mut ModifyReport,
) -> IteMrootResult<()> {
    if !visited.insert(root) {
        return Ok(());
    }

    let value = graph.vertex(root)?.value;
    if matches!(value, IteValue::Zero | IteValue::One | IteValue::Literal) {
        return Ok(());
    }
    if value != IteValue::IfThenElse {
        return Err(IteMrootError::ExpectedIfThenElse {
            vertex: root,
            value,
        });
    }

    let if_child = graph.child(root, IteChild::If)?;
    if graph.vertex(if_child)?.value == IteValue::Literal {
        let fanin = graph.positive_literal_fanin(if_child)?;
        if network.node_kind(fanin)? != NodeKind::PrimaryInput {
            graph.set_child(root, IteChild::If, network.ite_root(fanin)?)?;
            free_candidates.insert(if_child);
            report.replaced_children += 1;
        } else {
            visited.insert(if_child);
        }
    } else if graph.is_buffer(if_child)? {
        modify_part(
            network,
            graph,
            root,
            if_child,
            IteChild::If,
            visited,
            free_candidates,
            report,
        )?;
    } else {
        modify_fields_ite(network, graph, if_child, visited, free_candidates, report)?;
    }

    for child_slot in [IteChild::Then, IteChild::Else] {
        let child = graph.child(root, child_slot)?;
        if graph.is_buffer(child)? {
            modify_part(
                network,
                graph,
                root,
                child,
                child_slot,
                visited,
                free_candidates,
                report,
            )?;
        } else {
            modify_fields_ite(network, graph, child, visited, free_candidates, report)?;
        }
    }

    Ok(())
}

pub fn modify_part(
    network: &SisNetwork,
    graph: &mut IteGraph,
    parent: IteVertexId,
    child: IteVertexId,
    child_slot: IteChild,
    visited: &mut HashSet<IteVertexId>,
    free_candidates: &mut HashSet<IteVertexId>,
    report: &mut ModifyReport,
) -> IteMrootResult<()> {
    let child_if = graph.child(child, IteChild::If)?;
    let fanin = graph.positive_literal_fanin(child_if)?;

    if network.node_kind(fanin)? != NodeKind::PrimaryInput {
        for candidate_slot in [IteChild::If, IteChild::Then, IteChild::Else] {
            free_candidates.insert(graph.child(child, candidate_slot)?);
        }
        free_candidates.insert(child);
        graph.set_child(parent, child_slot, network.ite_root(fanin)?)?;
        report.replaced_children += 1;
    } else {
        visited.insert(child);
        for candidate_slot in [IteChild::If, IteChild::Then, IteChild::Else] {
            visited.insert(graph.child(child, candidate_slot)?);
        }
    }

    Ok(())
}

pub fn make_tree_and_map_with_mapper<Mapper>(
    network: &SisNetwork,
    graph: &mut IteGraph,
    mut mapper: Mapper,
) -> IteMrootResult<MappingReport>
where
    Mapper: FnMut(IteVertexId, &mut IteGraph) -> IteMrootResult<i32>,
{
    let mut roots = network.primary_output_roots()?;
    initialize_ite_area_network(graph, &mut roots)?;

    let mut mapped_in_order = Vec::new();
    let mut total_mux_structures = 0;
    for root in roots.iter().rev().copied() {
        total_mux_structures += mapper(root, graph)?;
        mapped_in_order.push(root);
    }

    Ok(MappingReport {
        roots,
        mapped_in_order,
        total_mux_structures,
    })
}

pub fn initialize_ite_area_network(
    graph: &mut IteGraph,
    multiple_fo_array: &mut Vec<IteVertexId>,
) -> IteMrootResult<()> {
    let init_num = multiple_fo_array.len();
    let mut table = HashSet::new();

    for root in multiple_fo_array.iter().copied().take(init_num) {
        table.insert(root);
        graph.reset_mapping_fields(root, 1)?;
    }

    for index in 0..init_num {
        let root = multiple_fo_array[index];
        if graph.vertex(root)?.value == IteValue::IfThenElse {
            for child in [IteChild::If, IteChild::Then, IteChild::Else] {
                let child_id = graph.child(root, child)?;
                initialize_area_ite(graph, child_id, multiple_fo_array, &mut table)?;
            }
        }
    }

    Ok(())
}

pub fn initialize_area_ite(
    graph: &mut IteGraph,
    vertex: IteVertexId,
    multiple_fo_array: &mut Vec<IteVertexId>,
    table: &mut HashSet<IteVertexId>,
) -> IteMrootResult<()> {
    if table.contains(&vertex) {
        let vertex_ref = graph.vertex_mut(vertex)?;
        if vertex_ref.multiple_fo == 0 {
            multiple_fo_array.push(vertex);
        }
        vertex_ref.multiple_fo += 1;
        vertex_ref.multiple_fo_for_mapping += 1;
        return Ok(());
    }

    table.insert(vertex);
    graph.reset_mapping_fields(vertex, 0)?;

    if graph.vertex(vertex)?.value != IteValue::IfThenElse {
        return Ok(());
    }

    for child in [IteChild::If, IteChild::Then, IteChild::Else] {
        let child_id = graph.child(vertex, child)?;
        initialize_area_ite(graph, child_id, multiple_fo_array, table)?;
    }

    Ok(())
}

pub fn traverse_and_free(network: &SisNetwork, graph: &mut IteGraph) -> IteMrootResult<FreeReport> {
    let roots = network.primary_output_roots()?;
    let mut visited = HashSet::new();
    let mut report = FreeReport::default();

    for root in roots {
        traverse_ite_for_free(graph, root, &mut visited, &mut report.freed_vertices)?;
    }

    Ok(report)
}

pub fn traverse_ite_for_free(
    graph: &mut IteGraph,
    vertex: IteVertexId,
    visited: &mut HashSet<IteVertexId>,
    freed: &mut Vec<IteVertexId>,
) -> IteMrootResult<()> {
    if !visited.insert(vertex) {
        return Ok(());
    }

    if graph.vertex(vertex)?.value == IteValue::IfThenElse {
        for child in [IteChild::If, IteChild::Then, IteChild::Else] {
            let child_id = graph.child(vertex, child)?;
            traverse_ite_for_free(graph, child_id, visited, freed)?;
        }
    }

    if graph.mark_freed(vertex)? {
        freed.push(vertex);
    }
    Ok(())
}

fn collect_and_mark_freed(
    graph: &mut IteGraph,
    root: IteVertexId,
    freed: &mut Vec<IteVertexId>,
) -> IteMrootResult<()> {
    let mut visited = HashSet::new();
    traverse_ite_for_free(graph, root, &mut visited, freed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_buffer(graph: &mut IteGraph, fanin: NodeId) -> IteVertexId {
        let zero = graph.add_vertex(IteVertex::terminal(0, false));
        let one = graph.add_vertex(IteVertex::terminal(0, true));
        let literal = graph.add_vertex(IteVertex::literal(0, fanin, true));
        graph.add_vertex(IteVertex::ite(0, literal, one, zero))
    }

    fn add_mux(
        graph: &mut IteGraph,
        selector: NodeId,
        then_child: IteVertexId,
        else_child: IteVertexId,
    ) -> IteVertexId {
        let literal = graph.add_vertex(IteVertex::literal(0, selector, true));
        graph.add_vertex(IteVertex::ite(0, literal, then_child, else_child))
    }

    #[test]
    fn buffer_detection_matches_c_predicate() {
        let mut graph = IteGraph::new();
        let buffer = add_buffer(&mut graph, NodeId(1));
        assert!(graph.is_buffer(buffer).unwrap());

        let not_buffer = add_mux(&mut graph, NodeId(1), buffer, buffer);
        assert!(!graph.is_buffer(not_buffer).unwrap());
    }

    #[test]
    fn modify_fields_node_replaces_internal_buffer_root_with_fanin_ite() {
        let mut graph = IteGraph::new();
        let one = graph.add_vertex(IteVertex::terminal(0, true));
        let zero = graph.add_vertex(IteVertex::terminal(0, false));
        let fanin_root = add_mux(&mut graph, NodeId(1), one, zero);
        let buffer = add_buffer(&mut graph, NodeId(2));

        let mut network = SisNetwork::new();
        network.add_node(SisNode::new(1, "a", NodeKind::PrimaryInput));
        network.add_node(SisNode::new(2, "n1", NodeKind::Internal).with_ite_root(fanin_root));
        network.add_node(SisNode::new(3, "n2", NodeKind::Internal).with_ite_root(buffer));

        let report = modify_fields_node(&mut network, &mut graph, NodeId(3)).unwrap();

        assert_eq!(network.ite_root(NodeId(3)).unwrap(), fanin_root);
        assert_eq!(report.replaced_roots, 1);
        assert!(graph.vertex(buffer).unwrap().freed);
    }

    #[test]
    fn modify_fields_ite_replaces_buffer_children_but_keeps_primary_input_buffers() {
        let mut graph = IteGraph::new();
        let internal_replacement = add_buffer(&mut graph, NodeId(1));
        let internal_buffer = add_buffer(&mut graph, NodeId(2));
        let pi_buffer = add_buffer(&mut graph, NodeId(1));
        let root = add_mux(&mut graph, NodeId(1), internal_buffer, pi_buffer);

        let mut network = SisNetwork::new();
        network.add_node(SisNode::new(1, "a", NodeKind::PrimaryInput));
        network.add_node(
            SisNode::new(2, "n1", NodeKind::Internal).with_ite_root(internal_replacement),
        );
        network.add_node(SisNode::new(3, "n2", NodeKind::Internal).with_ite_root(root));

        let report = modify_fields_node(&mut network, &mut graph, NodeId(3)).unwrap();

        assert_eq!(
            graph.child(root, IteChild::Then).unwrap(),
            internal_replacement
        );
        assert_eq!(graph.child(root, IteChild::Else).unwrap(), pi_buffer);
        assert_eq!(report.replaced_children, 1);
        assert!(graph.vertex(internal_buffer).unwrap().freed);
        assert!(!graph.vertex(pi_buffer).unwrap().freed);
    }

    #[test]
    fn initialize_area_tracks_multiple_fanout_vertices_and_mapper_order() {
        let mut graph = IteGraph::new();
        let zero = graph.add_vertex(IteVertex::terminal(0, false));
        let one = graph.add_vertex(IteVertex::terminal(0, true));
        let shared = add_mux(&mut graph, NodeId(1), one, zero);
        let root_a = add_mux(&mut graph, NodeId(1), shared, zero);
        let root_b = add_mux(&mut graph, NodeId(1), one, shared);

        let mut network = SisNetwork::new();
        network.add_node(SisNode::new(1, "a", NodeKind::PrimaryInput));
        network.add_node(SisNode::new(2, "n1", NodeKind::Internal).with_ite_root(root_a));
        network.add_node(SisNode::new(3, "n2", NodeKind::Internal).with_ite_root(root_b));
        network.add_node(SisNode::new(4, "po1", NodeKind::PrimaryOutput).with_fanins([NodeId(2)]));
        network.add_node(SisNode::new(5, "po2", NodeKind::PrimaryOutput).with_fanins([NodeId(3)]));

        let report = make_tree_and_map_with_mapper(&network, &mut graph, |root, graph| {
            Ok(match graph.vertex(root).unwrap().value {
                IteValue::IfThenElse if root == shared => 7,
                IteValue::IfThenElse => 2,
                _ => 0,
            })
        })
        .unwrap();

        assert_eq!(report.roots, vec![root_a, root_b, zero, one, shared]);
        assert_eq!(
            report.mapped_in_order,
            vec![shared, one, zero, root_b, root_a]
        );
        assert_eq!(report.total_mux_structures, 11);
        assert_eq!(graph.vertex(root_a).unwrap().multiple_fo, 1);
        assert_eq!(graph.vertex(root_b).unwrap().multiple_fo, 1);
        assert_eq!(graph.vertex(shared).unwrap().multiple_fo, 1);
        assert_eq!(graph.vertex(shared).unwrap().multiple_fo_for_mapping, 1);
    }

    #[test]
    fn traverse_and_free_de_duplicates_shared_output_roots() {
        let mut graph = IteGraph::new();
        let zero = graph.add_vertex(IteVertex::terminal(0, false));
        let one = graph.add_vertex(IteVertex::terminal(0, true));
        let root = add_mux(&mut graph, NodeId(1), one, zero);

        let mut network = SisNetwork::new();
        network.add_node(SisNode::new(1, "a", NodeKind::PrimaryInput));
        network.add_node(SisNode::new(2, "n1", NodeKind::Internal).with_ite_root(root));
        network.add_node(SisNode::new(3, "po1", NodeKind::PrimaryOutput).with_fanins([NodeId(2)]));
        network.add_node(SisNode::new(4, "po2", NodeKind::PrimaryOutput).with_fanins([NodeId(2)]));

        let report = traverse_and_free(&network, &mut graph).unwrap();

        assert_eq!(report.freed_vertices.len(), 4);
        assert!(graph.vertex(root).unwrap().freed);
        assert!(graph.vertex(one).unwrap().freed);
        assert!(graph.vertex(zero).unwrap().freed);
    }

    #[test]
    fn builder_flow_constructs_then_modifies_dfs_nodes() {
        let mut graph = IteGraph::new();
        let mut network = SisNetwork::new();
        network.add_node(SisNode::new(1, "a", NodeKind::PrimaryInput));
        network.add_node(SisNode::new(2, "n1", NodeKind::Internal));
        network.add_node(SisNode::new(3, "n2", NodeKind::Internal));

        let report = create_mroot_ite_network_with_builder(
            &mut network,
            &mut graph,
            &(),
            |node, _, network, graph| match network.node_kind(node)? {
                NodeKind::PrimaryInput | NodeKind::PrimaryOutput => Ok(None),
                NodeKind::Internal if node == NodeId(2) => Ok(Some(add_buffer(graph, NodeId(1)))),
                NodeKind::Internal => Ok(Some(add_buffer(graph, NodeId(2)))),
            },
        )
        .unwrap();

        assert_eq!(
            network.ite_root(NodeId(3)).unwrap(),
            network.ite_root(NodeId(2)).unwrap()
        );
        assert_eq!(report.replaced_roots, 1);
    }
}
