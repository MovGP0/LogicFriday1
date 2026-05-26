//! Native ASTG net reduction support.
//!
//! The reducer enumerates state-machine and marked-graph allocations at
//! vertices whose relevant degree is greater than one, removes the unselected
//! side of each allocation with the same cascading rule as SIS, then
//! canonicalizes the surviving weakly connected components.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AstgVertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgVertexKind
{
    Place,
    Transition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgVertex
{
    pub name: String,
    pub kind: AstgVertexKind,
}

impl AstgVertex
{
    pub fn new(name: impl Into<String>, kind: AstgVertexKind) -> Self
    {
        Self
        {
            name: name.into(),
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AstgEdge
{
    pub tail: AstgVertexId,
    pub head: AstgVertexId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgReductionGraph
{
    name: String,
    vertices: Vec<AstgVertex>,
    edges: Vec<AstgEdge>,
}

impl AstgReductionGraph
{
    pub fn new(name: impl Into<String>) -> Self
    {
        Self
        {
            name: name.into(),
            vertices: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn name(&self) -> &str
    {
        &self.name
    }

    pub fn add_vertex(&mut self, name: impl Into<String>, kind: AstgVertexKind) -> AstgVertexId
    {
        let id = AstgVertexId(self.vertices.len());
        self.vertices.push(AstgVertex::new(name, kind));
        id
    }

    pub fn add_edge(
        &mut self,
        tail: AstgVertexId,
        head: AstgVertexId,
    ) -> AstgReduceResult<()>
    {
        self.vertex(tail)?;
        self.vertex(head)?;
        self.edges.push(AstgEdge
        {
            tail,
            head,
        });
        Ok(())
    }

    pub fn vertex(&self, id: AstgVertexId) -> AstgReduceResult<&AstgVertex>
    {
        self.vertices
            .get(id.0)
            .ok_or(AstgReduceError::UnknownVertex(id))
    }

    pub fn vertices(&self) -> &[AstgVertex]
    {
        &self.vertices
    }

    pub fn edges(&self) -> &[AstgEdge]
    {
        &self.edges
    }

    fn validate(&self) -> AstgReduceResult<()>
    {
        for edge in &self.edges
        {
            self.vertex(edge.tail)?;
            self.vertex(edge.head)?;
        }

        Ok(())
    }

    fn in_neighbors(&self, vertex: AstgVertexId) -> Vec<AstgVertexId>
    {
        self.edges
            .iter()
            .filter_map(|edge| (edge.head == vertex).then_some(edge.tail))
            .collect()
    }

    fn out_neighbors(&self, vertex: AstgVertexId) -> Vec<AstgVertexId>
    {
        self.edges
            .iter()
            .filter_map(|edge| (edge.tail == vertex).then_some(edge.head))
            .collect()
    }

    fn weak_neighbors(&self, vertex: AstgVertexId) -> Vec<AstgVertexId>
    {
        let mut neighbors = self.in_neighbors(vertex);
        for neighbor in self.out_neighbors(vertex)
        {
            if !neighbors.contains(&neighbor)
            {
                neighbors.push(neighbor);
            }
        }

        neighbors
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgReduction
{
    pub kind: AstgReductionKind,
    pub status: AstgReductionStatus,
    pub upper_bound: usize,
    pub tried_allocations: usize,
    pub components: Vec<Vec<AstgVertexId>>,
    pub uncovered_vertices: Vec<AstgVertexId>,
    pub non_strong_components: Vec<usize>,
}

impl AstgReduction
{
    pub fn component_names(&self, graph: &AstgReductionGraph) -> AstgReduceResult<Vec<Vec<String>>>
    {
        self.components
            .iter()
            .map(|component| {
                component
                    .iter()
                    .map(|vertex| Ok(graph.vertex(*vertex)?.name.clone()))
                    .collect()
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgReductionKind
{
    StateMachine,
    MarkedGraph,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgReductionStatus
{
    Ok,
    NotCover,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstgReduceError
{
    UnknownVertex(AstgVertexId),
    UnexpectedVertexKind
    {
        vertex: AstgVertexId,
        expected: AstgVertexKind,
        actual: AstgVertexKind,
    },
    RecursionLimit
    {
        vertex: AstgVertexId,
        depth: usize,
    },
}

impl fmt::Display for AstgReduceError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::UnknownVertex(vertex) => write!(formatter, "unknown ASTG vertex {}", vertex.0),
            Self::UnexpectedVertexKind
            {
                vertex,
                expected,
                actual,
            } => write!(
                formatter,
                "ASTG vertex {} has kind {:?}, expected {:?}",
                vertex.0, actual, expected
            ),
            Self::RecursionLimit
            {
                vertex,
                depth,
            } => write!(
                formatter,
                "ASTG reduction cascade exceeded recursion limit at vertex {} depth {}",
                vertex.0, depth
            ),
        }
    }
}

impl Error for AstgReduceError {}

pub type AstgReduceResult<T> = Result<T, AstgReduceError>;

pub fn state_machine_components(graph: &AstgReductionGraph) -> AstgReduceResult<AstgReduction>
{
    find_reductions(graph, AstgVertexKind::Transition, AstgReductionKind::StateMachine)
}

pub fn marked_graph_components(graph: &AstgReductionGraph) -> AstgReduceResult<AstgReduction>
{
    find_reductions(graph, AstgVertexKind::Place, AstgReductionKind::MarkedGraph)
}

pub fn format_component(
    index: usize,
    graph: &AstgReductionGraph,
    component: &[AstgVertexId],
) -> AstgReduceResult<String>
{
    let mut text = format!("   {index})");

    for vertex in component
    {
        text.push(' ');
        text.push_str(&graph.vertex(*vertex)?.name);
    }

    Ok(text)
}

fn find_reductions(
    graph: &AstgReductionGraph,
    key_kind: AstgVertexKind,
    reduction_kind: AstgReductionKind,
) -> AstgReduceResult<AstgReduction>
{
    graph.validate()?;

    let mut state = ReductionState::new(graph.vertices.len());
    let (keys, upper_bound) = key_vertices(graph, key_kind, &mut state)?;
    let mut components = Vec::new();

    match reduction_kind
    {
        AstgReductionKind::StateMachine =>
        {
            generate_state_machine_allocations(graph, &keys, keys.len(), &mut state, &mut components)?;
        }
        AstgReductionKind::MarkedGraph =>
        {
            generate_marked_graph_allocations(graph, &keys, keys.len(), &mut state, &mut components)?;
        }
    }

    let (status, uncovered_vertices, non_strong_components) =
        check_reductions(graph, &components);

    Ok(AstgReduction
    {
        kind: reduction_kind,
        status,
        upper_bound,
        tried_allocations: state.tried_allocations,
        components,
        uncovered_vertices,
        non_strong_components,
    })
}

fn key_vertices(
    graph: &AstgReductionGraph,
    key_kind: AstgVertexKind,
    state: &mut ReductionState,
) -> AstgReduceResult<(Vec<AstgVertexId>, usize)>
{
    let mut keys = Vec::new();
    let mut upper_bound = 1usize;

    for (index, vertex) in graph.vertices.iter().enumerate()
    {
        if vertex.kind != key_kind
        {
            continue;
        }

        let id = AstgVertexId(index);
        let degree = match key_kind
        {
            AstgVertexKind::Transition => graph.in_neighbors(id).len(),
            AstgVertexKind::Place => graph.out_neighbors(id).len(),
        };

        if degree < 2
        {
            continue;
        }

        upper_bound = upper_bound.saturating_mul(degree);
        keys.push(id);

        match key_kind
        {
            AstgVertexKind::Transition =>
            {
                for predecessor in graph.in_neighbors(id)
                {
                    state.not_in_allocation[predecessor.0] = true;
                }
            }
            AstgVertexKind::Place =>
            {
                for successor in graph.out_neighbors(id)
                {
                    state.not_in_allocation[successor.0] = true;
                }
            }
        }
    }

    Ok((keys, upper_bound))
}

fn generate_state_machine_allocations(
    graph: &AstgReductionGraph,
    keys: &[AstgVertexId],
    remaining: usize,
    state: &mut ReductionState,
    components: &mut Vec<Vec<AstgVertexId>>,
) -> AstgReduceResult<()>
{
    if remaining == 0
    {
        do_state_machine_reduce(graph, state, components)?;
        return Ok(());
    }

    let transition = keys[remaining - 1];
    ensure_kind(graph, transition, AstgVertexKind::Transition)?;

    for place in graph.in_neighbors(transition)
    {
        state.not_in_allocation[place.0] = false;
        generate_state_machine_allocations(graph, keys, remaining - 1, state, components)?;
        state.not_in_allocation[place.0] = true;
    }

    Ok(())
}

fn generate_marked_graph_allocations(
    graph: &AstgReductionGraph,
    keys: &[AstgVertexId],
    remaining: usize,
    state: &mut ReductionState,
    components: &mut Vec<Vec<AstgVertexId>>,
) -> AstgReduceResult<()>
{
    if remaining == 0
    {
        do_marked_graph_reduce(graph, state, components)?;
        return Ok(());
    }

    let place = keys[remaining - 1];
    ensure_kind(graph, place, AstgVertexKind::Place)?;

    for transition in graph.out_neighbors(place)
    {
        state.not_in_allocation[transition.0] = false;
        generate_marked_graph_allocations(graph, keys, remaining - 1, state, components)?;
        state.not_in_allocation[transition.0] = true;
    }

    Ok(())
}

fn do_state_machine_reduce(
    graph: &AstgReductionGraph,
    state: &mut ReductionState,
    components: &mut Vec<Vec<AstgVertexId>>,
) -> AstgReduceResult<()>
{
    state.tried_allocations += 1;
    state.dead.fill(false);

    for index in 0..graph.vertices.len()
    {
        let transition = AstgVertexId(index);
        if state.dead[index]
        {
            continue;
        }

        for place in graph.in_neighbors(transition)
        {
            if !state.dead[place.0] && state.not_in_allocation[place.0]
            {
                delete_state_machine_place(graph, place, state, 0)?;
            }
        }
    }

    add_live_components(graph, &state.dead, components);
    Ok(())
}

fn do_marked_graph_reduce(
    graph: &AstgReductionGraph,
    state: &mut ReductionState,
    components: &mut Vec<Vec<AstgVertexId>>,
) -> AstgReduceResult<()>
{
    state.tried_allocations += 1;
    state.dead.fill(false);

    for index in 0..graph.vertices.len()
    {
        let place = AstgVertexId(index);
        if state.dead[index]
        {
            continue;
        }

        for transition in graph.out_neighbors(place)
        {
            if !state.dead[transition.0] && state.not_in_allocation[transition.0]
            {
                delete_marked_graph_transition(graph, transition, state, 0)?;
            }
        }
    }

    add_live_components(graph, &state.dead, components);
    Ok(())
}

fn delete_marked_graph_transition(
    graph: &AstgReductionGraph,
    transition: AstgVertexId,
    state: &mut ReductionState,
    depth: usize,
) -> AstgReduceResult<()>
{
    if state.dead[transition.0]
    {
        return Ok(());
    }

    let depth = depth + 1;
    if depth > 40
    {
        return Err(AstgReduceError::RecursionLimit
        {
            vertex: transition,
            depth,
        });
    }

    state.dead[transition.0] = true;

    for place in graph.out_neighbors(transition)
    {
        if state.dead[place.0]
        {
            continue;
        }

        if undead_fanin_count(graph, place, &state.dead) == 0
        {
            for successor in graph.out_neighbors(place)
            {
                delete_marked_graph_transition(graph, successor, state, depth)?;
            }

            state.dead[place.0] = true;
        }
    }

    Ok(())
}

fn delete_state_machine_place(
    graph: &AstgReductionGraph,
    place: AstgVertexId,
    state: &mut ReductionState,
    depth: usize,
) -> AstgReduceResult<()>
{
    if state.dead[place.0]
    {
        return Ok(());
    }

    let depth = depth + 1;
    if depth > 40
    {
        return Err(AstgReduceError::RecursionLimit
        {
            vertex: place,
            depth,
        });
    }

    state.dead[place.0] = true;

    for transition in graph.in_neighbors(place)
    {
        if state.dead[transition.0]
        {
            continue;
        }

        if undead_fanout_count(graph, transition, &state.dead) == 0
        {
            for predecessor in graph.in_neighbors(transition)
            {
                delete_state_machine_place(graph, predecessor, state, depth)?;
            }

            state.dead[transition.0] = true;
        }
    }

    Ok(())
}

fn undead_fanin_count(
    graph: &AstgReductionGraph,
    vertex: AstgVertexId,
    dead: &[bool],
) -> usize
{
    graph
        .in_neighbors(vertex)
        .into_iter()
        .filter(|predecessor| !dead[predecessor.0])
        .count()
}

fn undead_fanout_count(
    graph: &AstgReductionGraph,
    vertex: AstgVertexId,
    dead: &[bool],
) -> usize
{
    graph
        .out_neighbors(vertex)
        .into_iter()
        .filter(|successor| !dead[successor.0])
        .count()
}

fn add_live_components(
    graph: &AstgReductionGraph,
    dead: &[bool],
    components: &mut Vec<Vec<AstgVertexId>>,
)
{
    let mut visited = vec![false; graph.vertices.len()];

    for index in 0..graph.vertices.len()
    {
        if dead[index] || visited[index]
        {
            continue;
        }

        let mut component = Vec::new();
        let mut stack = vec![AstgVertexId(index)];
        visited[index] = true;

        while let Some(vertex) = stack.pop()
        {
            component.push(vertex);

            for neighbor in graph.weak_neighbors(vertex)
            {
                if !dead[neighbor.0] && !visited[neighbor.0]
                {
                    visited[neighbor.0] = true;
                    stack.push(neighbor);
                }
            }
        }

        component.sort_unstable();

        if !components.iter().any(|existing| *existing == component)
        {
            components.push(component);
        }
    }
}

fn check_reductions(
    graph: &AstgReductionGraph,
    components: &[Vec<AstgVertexId>],
) -> (AstgReductionStatus, Vec<AstgVertexId>, Vec<usize>)
{
    let mut covered = vec![false; graph.vertices.len()];
    let mut non_strong_components = Vec::new();

    for (index, component) in components.iter().enumerate()
    {
        for vertex in component
        {
            covered[vertex.0] = true;
        }

        if strong_component_count(graph, component) != 1
        {
            non_strong_components.push(index);
        }
    }

    let uncovered_vertices = covered
        .iter()
        .enumerate()
        .filter_map(|(index, is_covered)| (!*is_covered).then_some(AstgVertexId(index)))
        .collect::<Vec<_>>();

    let status = if uncovered_vertices.is_empty() && non_strong_components.is_empty()
    {
        AstgReductionStatus::Ok
    }
    else
    {
        AstgReductionStatus::NotCover
    };

    (status, uncovered_vertices, non_strong_components)
}

fn strong_component_count(
    graph: &AstgReductionGraph,
    component: &[AstgVertexId],
) -> usize
{
    if component.is_empty()
    {
        return 0;
    }

    let mut in_subset = vec![false; graph.vertices.len()];
    for vertex in component
    {
        in_subset[vertex.0] = true;
    }

    let mut visited = vec![false; graph.vertices.len()];
    let mut order = Vec::new();

    for vertex in component
    {
        if !visited[vertex.0]
        {
            dfs_forward(graph, *vertex, &in_subset, &mut visited, &mut order);
        }
    }

    let mut visited_reverse = vec![false; graph.vertices.len()];
    let mut count = 0usize;

    while let Some(vertex) = order.pop()
    {
        if visited_reverse[vertex.0]
        {
            continue;
        }

        count += 1;
        dfs_reverse(graph, vertex, &in_subset, &mut visited_reverse);
    }

    count
}

fn dfs_forward(
    graph: &AstgReductionGraph,
    vertex: AstgVertexId,
    in_subset: &[bool],
    visited: &mut [bool],
    order: &mut Vec<AstgVertexId>,
)
{
    visited[vertex.0] = true;

    for successor in graph.out_neighbors(vertex)
    {
        if in_subset[successor.0] && !visited[successor.0]
        {
            dfs_forward(graph, successor, in_subset, visited, order);
        }
    }

    order.push(vertex);
}

fn dfs_reverse(
    graph: &AstgReductionGraph,
    vertex: AstgVertexId,
    in_subset: &[bool],
    visited: &mut [bool],
)
{
    visited[vertex.0] = true;

    for predecessor in graph.in_neighbors(vertex)
    {
        if in_subset[predecessor.0] && !visited[predecessor.0]
        {
            dfs_reverse(graph, predecessor, in_subset, visited);
        }
    }
}

fn ensure_kind(
    graph: &AstgReductionGraph,
    vertex: AstgVertexId,
    expected: AstgVertexKind,
) -> AstgReduceResult<()>
{
    let actual = graph.vertex(vertex)?.kind;
    if actual == expected
    {
        Ok(())
    }
    else
    {
        Err(AstgReduceError::UnexpectedVertexKind
        {
            vertex,
            expected,
            actual,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReductionState
{
    dead: Vec<bool>,
    not_in_allocation: Vec<bool>,
    tried_allocations: usize,
}

impl ReductionState
{
    fn new(vertex_count: usize) -> Self
    {
        Self
        {
            dead: vec![false; vertex_count],
            not_in_allocation: vec![false; vertex_count],
            tried_allocations: 0,
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn state_machine_components_enumerate_one_input_place_per_multi_input_transition()
    {
        let mut graph = AstgReductionGraph::new("sm");
        let p0 = graph.add_vertex("p0", AstgVertexKind::Place);
        let p1 = graph.add_vertex("p1", AstgVertexKind::Place);
        let t0 = graph.add_vertex("t0", AstgVertexKind::Transition);
        let t1 = graph.add_vertex("t1", AstgVertexKind::Transition);
        let t2 = graph.add_vertex("t2", AstgVertexKind::Transition);

        graph.add_edge(p0, t0).unwrap();
        graph.add_edge(t0, p0).unwrap();
        graph.add_edge(p1, t0).unwrap();
        graph.add_edge(t0, p1).unwrap();
        graph.add_edge(p0, t1).unwrap();
        graph.add_edge(t1, p0).unwrap();
        graph.add_edge(p1, t2).unwrap();
        graph.add_edge(t2, p1).unwrap();

        let reduction = state_machine_components(&graph).unwrap();
        let names = reduction.component_names(&graph).unwrap();

        assert_eq!(reduction.kind, AstgReductionKind::StateMachine);
        assert_eq!(reduction.upper_bound, 2);
        assert_eq!(reduction.tried_allocations, 2);
        assert_eq!(reduction.status, AstgReductionStatus::Ok);
        assert_eq!(
            names,
            vec![
                vec!["p0".to_owned(), "t0".to_owned(), "t1".to_owned()],
                vec!["p1".to_owned(), "t0".to_owned(), "t2".to_owned()],
            ]
        );
    }

    #[test]
    fn marked_graph_components_enumerate_one_output_transition_per_multi_output_place()
    {
        let mut graph = AstgReductionGraph::new("mg");
        let p0 = graph.add_vertex("p0", AstgVertexKind::Place);
        let p1 = graph.add_vertex("p1", AstgVertexKind::Place);
        let p2 = graph.add_vertex("p2", AstgVertexKind::Place);
        let t0 = graph.add_vertex("t0", AstgVertexKind::Transition);
        let t1 = graph.add_vertex("t1", AstgVertexKind::Transition);

        graph.add_edge(p0, t0).unwrap();
        graph.add_edge(t0, p0).unwrap();
        graph.add_edge(p0, t1).unwrap();
        graph.add_edge(t1, p0).unwrap();
        graph.add_edge(t0, p1).unwrap();
        graph.add_edge(p1, t0).unwrap();
        graph.add_edge(t1, p2).unwrap();
        graph.add_edge(p2, t1).unwrap();

        let reduction = marked_graph_components(&graph).unwrap();
        let names = reduction.component_names(&graph).unwrap();

        assert_eq!(reduction.kind, AstgReductionKind::MarkedGraph);
        assert_eq!(reduction.upper_bound, 2);
        assert_eq!(reduction.tried_allocations, 2);
        assert_eq!(reduction.status, AstgReductionStatus::Ok);
        assert_eq!(
            names,
            vec![
                vec!["p0".to_owned(), "p1".to_owned(), "t0".to_owned()],
                vec!["p0".to_owned(), "p2".to_owned(), "t1".to_owned()],
            ]
        );
    }

    #[test]
    fn check_reports_uncovered_vertices_and_non_strong_components()
    {
        let mut graph = AstgReductionGraph::new("uncovered");
        let p0 = graph.add_vertex("p0", AstgVertexKind::Place);
        let p1 = graph.add_vertex("p1", AstgVertexKind::Place);
        let t0 = graph.add_vertex("t0", AstgVertexKind::Transition);

        graph.add_edge(p0, t0).unwrap();
        let (status, uncovered_vertices, non_strong_components) =
            check_reductions(&graph, &[vec![p0, t0]]);

        assert_eq!(status, AstgReductionStatus::NotCover);
        assert_eq!(uncovered_vertices, vec![p1]);
        assert_eq!(non_strong_components, vec![0]);
    }

    #[test]
    fn component_format_matches_legacy_component_prefix()
    {
        let mut graph = AstgReductionGraph::new("format");
        let p0 = graph.add_vertex("p0", AstgVertexKind::Place);
        let t0 = graph.add_vertex("t0", AstgVertexKind::Transition);
        let text = format_component(3, &graph, &[p0, t0]).unwrap();

        assert_eq!(text, "   3) p0 t0");
    }
}
