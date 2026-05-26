//! Native Rust graph algorithms for ASTG nets.
//!
//! This module ports the graph predicates and traversal routines from the SIS
//! ASTG core without exposing legacy C ABI symbols.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EdgeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexKind
{
    Place,
    Transition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgVertex
{
    pub name: String,
    pub kind: VertexKind,
    pub subset: bool,
}

impl AstgVertex
{
    pub fn place(name: impl Into<String>) -> Self
    {
        Self::new(name, VertexKind::Place)
    }

    pub fn transition(name: impl Into<String>) -> Self
    {
        Self::new(name, VertexKind::Transition)
    }

    pub fn new(name: impl Into<String>, kind: VertexKind) -> Self
    {
        Self
        {
            name: name.into(),
            kind,
            subset: true,
        }
    }

    pub fn with_subset(mut self, subset: bool) -> Self
    {
        self.subset = subset;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgEdge
{
    pub tail: VertexId,
    pub head: VertexId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgGraph
{
    pub name: String,
    vertices: Vec<AstgVertex>,
    edges: Vec<AstgEdge>,
    out_edges: Vec<Vec<EdgeId>>,
    in_edges: Vec<Vec<EdgeId>>,
    slots: BTreeMap<AstgSlot, AstgSlotValue>,
}

impl AstgGraph
{
    pub fn new(name: impl Into<String>) -> Self
    {
        Self
        {
            name: name.into(),
            vertices: Vec::new(),
            edges: Vec::new(),
            out_edges: Vec::new(),
            in_edges: Vec::new(),
            slots: BTreeMap::new(),
        }
    }

    pub fn add_vertex(&mut self, vertex: AstgVertex) -> VertexId
    {
        let id = VertexId(self.vertices.len());
        self.vertices.push(vertex);
        self.out_edges.push(Vec::new());
        self.in_edges.push(Vec::new());
        id
    }

    pub fn add_place(&mut self, name: impl Into<String>) -> VertexId
    {
        self.add_vertex(AstgVertex::place(name))
    }

    pub fn add_transition(&mut self, name: impl Into<String>) -> VertexId
    {
        self.add_vertex(AstgVertex::transition(name))
    }

    pub fn add_edge(&mut self, tail: VertexId, head: VertexId) -> Result<EdgeId, AstgGraphError>
    {
        self.require_vertex(tail)?;
        self.require_vertex(head)?;

        let id = EdgeId(self.edges.len());
        self.edges.push(AstgEdge
        {
            tail, head
        }
        );
        self.out_edges[tail.0].push(id);
        self.in_edges[head.0].push(id);

        Ok(id)
    }

    pub fn vertex(&self, id: VertexId) -> Option<&AstgVertex>
    {
        self.vertices.get(id.0)
    }

    pub fn vertex_mut(&mut self, id: VertexId) -> Option<&mut AstgVertex>
    {
        self.vertices.get_mut(id.0)
    }

    pub fn vertices(&self) -> impl Iterator<Item = (VertexId, &AstgVertex)>
    {
        self.vertices
        .iter()
        .enumerate()
        .map(|(index, vertex)| (VertexId(index), vertex))
    }

    pub fn places(&self) -> impl Iterator<Item = (VertexId, &AstgVertex)>
    {
        self.vertices()
        .filter(|(_, vertex)| vertex.kind == VertexKind::Place)
    }

    pub fn transitions(&self) -> impl Iterator<Item = (VertexId, &AstgVertex)>
    {
        self.vertices()
        .filter(|(_, vertex)| vertex.kind == VertexKind::Transition)
    }

    pub fn vertex_count(&self) -> usize
    {
        self.vertices.len()
    }

    pub fn edge_count(&self) -> usize
    {
        self.edges.len()
    }

    pub fn in_degree(&self, id: VertexId) -> usize
    {
        self.in_edges.get(id.0).map_or(0, Vec::len)
    }

    pub fn out_degree(&self, id: VertexId) -> usize
    {
        self.out_edges.get(id.0).map_or(0, Vec::len)
    }

    pub fn output_vertices(&self, id: VertexId) -> Vec<VertexId>
    {
        self.out_edges
        .get(id.0)
        .into_iter()
        .flatten()
        .map(|edge| self.edges[edge.0].head)
        .collect()
    }

    pub fn input_vertices(&self, id: VertexId) -> Vec<VertexId>
    {
        self.in_edges
        .get(id.0)
        .into_iter()
        .flatten()
        .map(|edge| self.edges[edge.0].tail)
        .collect()
    }

    pub fn set_subset(&mut self, id: VertexId, subset: bool) -> Result<(), AstgGraphError>
    {
        self.vertex_mut(id)
        .ok_or(AstgGraphError::UnknownVertex(id))?
        .subset = subset;
        Ok(())
    }

    pub fn set_slot(&mut self, slot: AstgSlot, value: AstgSlotValue)
    {
        self.slots.insert(slot, value);
    }

    pub fn get_slot(&self, slot: AstgSlot) -> Option<&AstgSlotValue>
    {
        self.slots.get(&slot)
    }

    fn require_vertex(&self, id: VertexId) -> Result<(), AstgGraphError>
    {
        if id.0 < self.vertices.len()
        {
            Ok(())
        }
        else
        {
            Err(AstgGraphError::UnknownVertex(id))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AstgSlot(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstgSlotValue
{
    Integer(i64),
    Text(String),
    Vertex(VertexId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstgGraphError
{
    UnknownVertex(VertexId),
}

impl fmt::Display for AstgGraphError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self
        {
            Self::UnknownVertex(id) => write!(formatter, "unknown ASTG vertex {}", id.0),
        }
    }
}

impl Error for AstgGraphError
{
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgDaemonType
{
    Alloc,
    Duplicate,
    Invalid,
    Free,
}

pub type AstgDaemon = Box<dyn FnMut(&AstgGraph, Option<&AstgGraph>) + Send>;

pub struct AstgDaemonRegistry
{
    daemons: Vec<AstgDaemonEntry>,
}

struct AstgDaemonEntry
{
    daemon_type: AstgDaemonType,
    daemon: AstgDaemon,
}

impl AstgDaemonRegistry
{
    pub fn new() -> Self
    {
        Self
        {
            daemons: Vec::new(),
        }
    }

    pub fn register<F>(&mut self, daemon_type: AstgDaemonType, daemon: F)
    where
    F: FnMut(&AstgGraph, Option<&AstgGraph>) + Send + 'static,
    {
        self.daemons.push(AstgDaemonEntry
        {
            daemon_type,
            daemon: Box::new(daemon),
        }
        );
    }

    pub fn run(&mut self, daemon_type: AstgDaemonType, stg1: &AstgGraph, stg2: Option<&AstgGraph>)
    {
        for entry in &mut self.daemons
        {
            if entry.daemon_type == daemon_type
            {
                (entry.daemon)(stg1, stg2);
            }
        }
    }

    pub fn discard(&mut self)
    {
        self.daemons.clear();
    }

    pub fn len(&self) -> usize
    {
        self.daemons.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.daemons.is_empty()
    }
}

impl Default for AstgDaemonRegistry
{
    fn default() -> Self
    {
        Self::new()
    }
}

pub fn astg_is_marked_graph(stg: &AstgGraph) -> bool
{
    stg.places()
    .all(|(id, _)| stg.in_degree(id) <= 1 && stg.out_degree(id) <= 1)
}

pub fn astg_is_state_machine(stg: &AstgGraph) -> bool
{
    stg.transitions()
    .all(|(id, _)| stg.in_degree(id) <= 1 && stg.out_degree(id) <= 1)
}

pub fn astg_is_free_choice_net(stg: &AstgGraph) -> bool
{
    stg.places().all(|(place, _)|
    {
        stg.out_degree(place) <= 1
        || stg
        .output_vertices(place)
        .into_iter()
        .all(|transition| stg.in_degree(transition) == 1)
    }
    )
}

pub fn astg_is_place_simple(stg: &AstgGraph) -> bool
{
    for (source, _) in stg.transitions()
    {
        let mut seen = HashSet::new();

        for place in stg.output_vertices(source)
        {
            for destination in stg.output_vertices(place)
            {
                if !seen.insert(destination)
                {
                    return false;
                }
            }
        }
    }

    true
}

pub fn astg_is_pure(stg: &AstgGraph) -> bool
{
    stg.places().all(|(place, _)|
    {
        stg.output_vertices(place).into_iter().all(|transition|
        {
            stg.output_vertices(transition)
            .into_iter()
            .all(|next_place| next_place != place)
        }
        )
    }
    )
}

pub fn astg_simple_cycles<F>(
stg: &AstgGraph,
target: Option<VertexId>,
subset: bool,
mut callback: F,
) -> usize
where
F: FnMut(&[VertexId]) -> bool,
{
    let mut reported = BTreeSet::new();
    let mut count = 0;
    let starts = match target
    {
        Some(id) => vec![id],
        None => stg.vertices().map(|(id, _)| id).collect(),
    }
    ;

    for start in starts
    {
        if !vertex_is_in_scope(stg, start, subset)
        {
            continue;
        }

        let mut path = Vec::new();
        let mut active = BTreeSet::new();
        simple_cycles_dfs(
        stg,
        start,
        start,
        subset,
        &mut path,
        &mut active,
        &mut reported,
        &mut count,
        &mut callback,
        );
    }

    count
}

pub fn astg_top_sort(stg: &AstgGraph, subset: bool) -> Vec<VertexId>
{
    let mut visited = vec![false; stg.vertex_count()];
    let mut result = Vec::new();

    for (vertex, _) in stg.vertices()
    {
        if !visited[vertex.0] && vertex_is_in_scope(stg, vertex, subset)
        {
            top_sort_dfs(stg, vertex, subset, &mut visited, &mut result);
        }
    }

    result
}

pub fn astg_connected_comp<F>(stg: &AstgGraph, subset: bool, mut callback: F) -> usize
where
F: FnMut(&[VertexId], usize),
{
    let mut visited = vec![false; stg.vertex_count()];
    let mut component_count = 0;

    for (vertex, _) in stg.vertices()
    {
        if visited[vertex.0] || !vertex_is_in_scope(stg, vertex, subset)
        {
            continue;
        }

        let mut component = Vec::new();
        connected_comp_dfs(stg, vertex, subset, &mut visited, &mut component);
        callback(&component, component_count);
        component_count += 1;
    }

    component_count
}

pub fn astg_strong_comp<F>(stg: &AstgGraph, subset: bool, mut callback: F) -> usize
where
F: FnMut(&[VertexId], usize),
{
    let order = astg_top_sort(stg, subset);
    let mut visited = vec![false; stg.vertex_count()];
    let mut component_count = 0;

    for vertex in order.into_iter().rev()
    {
        if visited[vertex.0] || !vertex_is_in_scope(stg, vertex, subset)
        {
            continue;
        }

        let mut component = Vec::new();
        strong_comp_dfs(stg, vertex, subset, &mut visited, &mut component);
        callback(&component, component_count);
        component_count += 1;
    }

    component_count
}

pub fn astg_has_cycles(stg: &AstgGraph) -> bool
{
    let mut state = vec![VisitState::Unvisited; stg.vertex_count()];

    for (vertex, _) in stg.vertices()
    {
        if state[vertex.0] == VisitState::Unvisited && find_cycle(stg, vertex, &mut state)
        {
            return true;
        }
    }

    false
}

pub fn astg_choose_path(
stg: &AstgGraph,
start: VertexId,
destination: VertexId,
) -> Option<Vec<VertexId>>
{
    if stg.vertex(start).is_none() || stg.vertex(destination).is_none()
    {
        return None;
    }

    let mut visited = vec![false; stg.vertex_count()];
    let mut path = Vec::new();

    if choose_path_dfs(stg, start, destination, &mut visited, &mut path)
    {
        Some(path)
    }
    else
    {
        None
    }
}

pub fn astg_print_path(stg: &AstgGraph, path: &[VertexId]) -> String
{
    let mut result = String::from("Path:");

    for vertex in path
    {
        if let Some(info) = stg.vertex(*vertex)
        {
            result.push(' ');
            result.push_str(&info.name);
        }
    }

    result.push('\n');
    result
}

pub fn astg_print(stg: &AstgGraph) -> String
{
    let mut result = format!("Graph '{}':\n", stg.name);

    for (vertex, info) in stg.vertices()
    {
        result.push_str("  ");
        result.push_str(&info.name);

        for output in stg.output_vertices(vertex)
        {
            if let Some(output_info) = stg.vertex(output)
            {
                result.push(' ');
                result.push_str(&output_info.name);
            }
        }

        result.push('\n');
    }

    result
}

fn simple_cycles_dfs<F>(
stg: &AstgGraph,
start: VertexId,
current: VertexId,
subset: bool,
path: &mut Vec<VertexId>,
active: &mut BTreeSet<VertexId>,
reported: &mut BTreeSet<Vec<VertexId>>,
count: &mut usize,
callback: &mut F,
) where
F: FnMut(&[VertexId]) -> bool,
{
    path.push(current);
    active.insert(current);

    for next in stg.output_vertices(current)
    {
        if !vertex_is_in_scope(stg, next, subset)
        {
            continue;
        }

        if next == start
        {
            let mut cycle = path.clone();
            cycle.push(start);
            let key = canonical_cycle_key(&cycle);

            if reported.insert(key) && callback(&cycle)
            {
                *count += 1;
            }
        }
        else if !active.contains(&next)
        {
            simple_cycles_dfs(
            stg, start, next, subset, path, active, reported, count, callback,
            );
        }
    }

    active.remove(&current);
    path.pop();
}

fn canonical_cycle_key(cycle: &[VertexId]) -> Vec<VertexId>
{
    let body = &cycle[..cycle.len().saturating_sub(1)];
    let mut best = body.to_vec();

    for offset in 1..body.len()
    {
        let rotated = body[offset..]
        .iter()
        .chain(body[..offset].iter())
        .copied()
        .collect::<Vec<_>>();

        if rotated < best
        {
            best = rotated;
        }
    }

    best
}

fn top_sort_dfs(
stg: &AstgGraph,
vertex: VertexId,
subset: bool,
visited: &mut [bool],
result: &mut Vec<VertexId>,
)
{
    if visited[vertex.0]
    {
        return;
    }

    visited[vertex.0] = true;

    for output in stg.output_vertices(vertex)
    {
        if vertex_is_in_scope(stg, output, subset)
        {
            top_sort_dfs(stg, output, subset, visited, result);
        }
    }

    result.push(vertex);
}

fn connected_comp_dfs(
stg: &AstgGraph,
vertex: VertexId,
subset: bool,
visited: &mut [bool],
component: &mut Vec<VertexId>,
)
{
    if visited[vertex.0] || !vertex_is_in_scope(stg, vertex, subset)
    {
        return;
    }

    visited[vertex.0] = true;
    component.push(vertex);

    for next in stg
    .input_vertices(vertex)
    .into_iter()
    .chain(stg.output_vertices(vertex))
    {
        connected_comp_dfs(stg, next, subset, visited, component);
    }
}

fn strong_comp_dfs(
stg: &AstgGraph,
vertex: VertexId,
subset: bool,
visited: &mut [bool],
component: &mut Vec<VertexId>,
)
{
    if visited[vertex.0] || !vertex_is_in_scope(stg, vertex, subset)
    {
        return;
    }

    visited[vertex.0] = true;
    component.push(vertex);

    for input in stg.input_vertices(vertex)
    {
        strong_comp_dfs(stg, input, subset, visited, component);
    }
}

fn find_cycle(stg: &AstgGraph, vertex: VertexId, state: &mut [VisitState]) -> bool
{
    match state[vertex.0]
    {
        VisitState::Active => return true,
        VisitState::Done => return false,
        VisitState::Unvisited =>
        {
        }
    }

    state[vertex.0] = VisitState::Active;

    for output in stg.output_vertices(vertex)
    {
        if find_cycle(stg, output, state)
        {
            return true;
        }
    }

    state[vertex.0] = VisitState::Done;
    false
}

fn choose_path_dfs(
stg: &AstgGraph,
vertex: VertexId,
destination: VertexId,
visited: &mut [bool],
path: &mut Vec<VertexId>,
) -> bool
{
    if visited[vertex.0]
    {
        return false;
    }

    visited[vertex.0] = true;
    path.push(vertex);

    if vertex == destination
    {
        return true;
    }

    for output in stg.output_vertices(vertex)
    {
        if choose_path_dfs(stg, output, destination, visited, path)
        {
            return true;
        }
    }

    path.pop();
    false
}

fn vertex_is_in_scope(stg: &AstgGraph, id: VertexId, subset: bool) -> bool
{
    stg.vertex(id)
    .is_some_and(|vertex| !subset || vertex.subset)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitState
{
    Unvisited,
    Active,
    Done,
}

#[cfg(test)]
mod tests
{
    use std::sync::
    {
        Arc, Mutex
    }
    ;

    use super::*;

    fn sample_net() -> (AstgGraph, VertexId, VertexId, VertexId, VertexId)
    {
        let mut graph = AstgGraph::new("sample");
        let t0 = graph.add_transition("t0");
        let p0 = graph.add_place("p0");
        let t1 = graph.add_transition("t1");
        let p1 = graph.add_place("p1");
        graph.add_edge(t0, p0).unwrap();
        graph.add_edge(p0, t1).unwrap();
        graph.add_edge(t1, p1).unwrap();
        graph.add_edge(p1, t0).unwrap();

        (graph, t0, p0, t1, p1)
    }

    #[test]
    fn daemon_registry_runs_matching_daemons_and_discards_them()
    {
        let graph = AstgGraph::new("g");
        let calls = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&calls);
        let mut registry = AstgDaemonRegistry::new();

        registry.register(AstgDaemonType::Alloc, move |new_graph, old_graph|
        {
            captured
            .lock()
            .unwrap()
            .push((new_graph.name.clone(), old_graph.is_none()));
        }
        );
        registry.register(AstgDaemonType::Free, |_new_graph, _old_graph|
        {
            panic!("wrong daemon type was invoked");
        }
        );

        registry.run(AstgDaemonType::Alloc, &graph, None);
        registry.discard();

        assert_eq!(&*calls.lock().unwrap(), &[("g".to_owned(), true)]);
        assert!(registry.is_empty());
    }

    #[test]
    fn slots_store_and_replace_native_values()
    {
        let mut graph = AstgGraph::new("g");
        graph.set_slot(AstgSlot(1), AstgSlotValue::Integer(7));
        graph.set_slot(AstgSlot(1), AstgSlotValue::Text("done".to_owned()));

        assert_eq!(
        graph.get_slot(AstgSlot(1)),
        Some(&AstgSlotValue::Text("done".to_owned()))
        );
        assert_eq!(graph.get_slot(AstgSlot(2)), None);
    }

    #[test]
    fn predicates_match_legacy_degree_checks()
    {
        let (mut graph, t0, p0, _t1, p1) = sample_net();

        assert!(astg_is_marked_graph(&graph));
        assert!(astg_is_state_machine(&graph));
        assert!(astg_is_free_choice_net(&graph));
        assert!(astg_is_place_simple(&graph));
        assert!(astg_is_pure(&graph));

        let extra = graph.add_transition("extra");
        graph.add_edge(p0, extra).unwrap();
        graph.add_edge(p1, extra).unwrap();
        graph.add_edge(t0, p0).unwrap();

        assert!(!astg_is_marked_graph(&graph));
        assert!(!astg_is_state_machine(&graph));
        assert!(!astg_is_free_choice_net(&graph));
    }

    #[test]
    fn detects_non_place_simple_and_impure_nets()
    {
        let mut graph = AstgGraph::new("g");
        let t0 = graph.add_transition("t0");
        let p0 = graph.add_place("p0");
        let p1 = graph.add_place("p1");
        let t1 = graph.add_transition("t1");
        graph.add_edge(t0, p0).unwrap();
        graph.add_edge(t0, p1).unwrap();
        graph.add_edge(p0, t1).unwrap();
        graph.add_edge(p1, t1).unwrap();

        assert!(!astg_is_place_simple(&graph));

        let mut graph = AstgGraph::new("pure");
        let p = graph.add_place("p");
        let t = graph.add_transition("t");
        graph.add_edge(p, t).unwrap();
        graph.add_edge(t, p).unwrap();

        assert!(!astg_is_pure(&graph));
    }

    #[test]
    fn cycle_detection_and_reporting_use_vertex_paths()
    {
        let (graph, t0, p0, t1, p1) = sample_net();
        let mut cycles = Vec::new();

        let count = astg_simple_cycles(&graph, None, false, |cycle|
        {
            cycles.push(cycle.to_vec());
            true
        }
        );

        assert!(astg_has_cycles(&graph));
        assert_eq!(count, 1);
        assert_eq!(cycles, vec![vec![t0, p0, t1, p1, t0]]);
    }

    #[test]
    fn top_sort_preserves_legacy_postorder_indices()
    {
        let mut graph = AstgGraph::new("dag");
        let a = graph.add_transition("a");
        let b = graph.add_place("b");
        let c = graph.add_transition("c");
        graph.add_edge(a, b).unwrap();
        graph.add_edge(b, c).unwrap();

        assert_eq!(astg_top_sort(&graph, false), vec![c, b, a]);
        assert!(!astg_has_cycles(&graph));
    }

    #[test]
    fn connected_components_walk_edges_as_undirected()
    {
        let (mut graph, t0, p0, t1, p1) = sample_net();
        let isolated = graph.add_place("isolated");
        let mut components = Vec::new();

        let count = astg_connected_comp(&graph, false, |component, _number|
        {
            components.push(component.iter().copied().collect::<BTreeSet<_>>());
        }
        );

        assert_eq!(count, 2);
        assert!(components.contains(&[t0, p0, t1, p1].into_iter().collect()));
        assert!(components.contains(&[isolated].into_iter().collect()));
    }

    #[test]
    fn strong_components_use_reverse_edges_after_top_sort()
    {
        let (mut graph, t0, p0, t1, p1) = sample_net();
        let tail = graph.add_transition("tail");
        graph.add_edge(p1, tail).unwrap();
        let mut components = Vec::new();

        let count = astg_strong_comp(&graph, false, |component, _number|
        {
            components.push(component.iter().copied().collect::<BTreeSet<_>>());
        }
        );

        assert_eq!(count, 2);
        assert!(components.contains(&[t0, p0, t1, p1].into_iter().collect()));
        assert!(components.contains(&[tail].into_iter().collect()));
    }

    #[test]
    fn subset_traversals_skip_unselected_vertices()
    {
        let (mut graph, t0, p0, t1, p1) = sample_net();
        graph.set_subset(p1, false).unwrap();

        assert_eq!(astg_top_sort(&graph, true), vec![t1, p0, t0]);
        assert!(!astg_has_cycles(&AstgGraph::new("empty")));
    }

    #[test]
    fn choose_path_returns_the_first_reachable_path()
    {
        let (graph, t0, p0, t1, _p1) = sample_net();

        assert_eq!(astg_choose_path(&graph, t0, t1), Some(vec![t0, p0, t1]));
        assert_eq!(astg_print_path(&graph, &[t0, p0, t1]), "Path: t0 p0 t1\n");
    }

    #[test]
    fn print_graph_lists_each_vertex_and_its_outputs()
    {
        let (graph, _t0, _p0, _t1, _p1) = sample_net();

        assert_eq!(
        astg_print(&graph),
        concat!(
        "Graph 'sample':\n",
        "  t0 p0\n",
        "  p0 t1\n",
        "  t1 p1\n",
        "  p1 t0\n",
        )
        );
    }

    #[test]
    fn source_contains_no_dependency_metadata_or_c_abi_exports()
    {
        let source = include_str!("astg_core2.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
