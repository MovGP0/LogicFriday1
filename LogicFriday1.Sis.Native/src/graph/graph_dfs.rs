//! Depth-first graph ordering for the native SIS graph substrate.
//!
//! The C implementation returns vertices in dependency-first DFS order and
//! reports a cyclic graph by failing. This Rust port keeps the ordering
//! semantics but exposes cycle and structural errors as ordinary results.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[cfg(not(test))]
use super::graph::{Graph, GraphError, VertexId};

#[cfg(test)]
#[path = "graph.rs"]
mod graph;

#[cfg(test)]
use graph::{Graph, GraphError, VertexId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DfsState {
    Visiting,
    Done,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphDfsError {
    Cycle { vertex: VertexId },
    Graph(GraphError),
}

impl fmt::Display for GraphDfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cycle { vertex } => write!(f, "graph has a cycle at vertex {:?}", vertex),
            Self::Graph(error) => write!(f, "{error}"),
        }
    }
}

impl Error for GraphDfsError {}

impl From<GraphError> for GraphDfsError {
    fn from(value: GraphError) -> Self {
        Self::Graph(value)
    }
}

pub fn depth_first_sort<G, V, E>(graph: &Graph<G, V, E>) -> Result<Vec<VertexId>, GraphDfsError> {
    let mut order = Vec::with_capacity(graph.vertex_count());
    let mut visited_by_end_search = HashSet::new();
    let mut dfs_states = HashMap::new();

    for vertex in graph.vertices() {
        if !dfs_states.contains_key(&vertex.id) {
            visited_by_end_search.insert(vertex.id);
            let end_vertex = find_end_vertex(graph, vertex.id, &mut visited_by_end_search)?;
            depth_first_recur(graph, end_vertex, &mut dfs_states, &mut order)?;
        }
    }

    Ok(order)
}

pub fn is_acyclic<G, V, E>(graph: &Graph<G, V, E>) -> bool {
    depth_first_sort(graph).is_ok()
}

fn find_end_vertex<G, V, E>(
    graph: &Graph<G, V, E>,
    start: VertexId,
    visited: &mut HashSet<VertexId>,
) -> Result<VertexId, GraphDfsError> {
    let mut current = start;

    loop {
        let vertex = graph.vertex(current)?;
        let Some(edge_id) = vertex.outgoing_edges().first().copied() else {
            return Ok(current);
        };

        let destination = graph.edge(edge_id)?.destination;
        if !visited.insert(destination) {
            return Err(GraphDfsError::Cycle {
                vertex: destination,
            });
        }

        current = destination;
    }
}

fn depth_first_recur<G, V, E>(
    graph: &Graph<G, V, E>,
    vertex: VertexId,
    states: &mut HashMap<VertexId, DfsState>,
    order: &mut Vec<VertexId>,
) -> Result<(), GraphDfsError> {
    match states.get(&vertex).copied() {
        Some(DfsState::Done) => return Ok(()),
        Some(DfsState::Visiting) => {
            return Err(GraphDfsError::Cycle { vertex });
        }
        None => {}
    }

    states.insert(vertex, DfsState::Visiting);

    for edge_id in graph.vertex(vertex)?.incoming_edges() {
        let source = graph.edge(*edge_id)?.source;
        depth_first_recur(graph, source, states, order)?;
    }

    states.insert(vertex, DfsState::Done);
    order.push(vertex);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> (Graph<(), usize, ()>, Vec<VertexId>) {
        let mut graph = Graph::new(());
        let vertices = (0..10)
            .map(|index| graph.add_vertex(index))
            .collect::<Vec<_>>();

        graph.add_edge(vertices[3], vertices[4], ()).unwrap();
        graph.add_edge(vertices[0], vertices[3], ()).unwrap();
        graph.add_edge(vertices[0], vertices[6], ()).unwrap();
        graph.add_edge(vertices[0], vertices[2], ()).unwrap();
        graph.add_edge(vertices[1], vertices[3], ()).unwrap();
        graph.add_edge(vertices[6], vertices[3], ()).unwrap();
        graph.add_edge(vertices[2], vertices[5], ()).unwrap();
        graph.add_edge(vertices[2], vertices[3], ()).unwrap();
        graph.add_edge(vertices[3], vertices[5], ()).unwrap();
        graph.add_edge(vertices[6], vertices[2], ()).unwrap();
        graph.add_edge(vertices[7], vertices[8], ()).unwrap();
        graph.add_edge(vertices[9], vertices[7], ()).unwrap();
        graph.add_edge(vertices[9], vertices[8], ()).unwrap();

        (graph, vertices)
    }

    #[test]
    fn depth_first_sort_matches_sis_graph_dfs_test_order() {
        let (graph, vertices) = sample_graph();

        let order = depth_first_sort(&graph).unwrap();
        let values = order
            .iter()
            .map(|id| graph.vertex(*id).unwrap().data)
            .collect::<Vec<_>>();

        assert_eq!(order.len(), vertices.len());
        assert_eq!(values, vec![0, 1, 6, 2, 3, 4, 5, 9, 7, 8]);
    }

    #[test]
    fn every_edge_source_precedes_destination_in_acyclic_sort() {
        let (graph, _) = sample_graph();
        let order = depth_first_sort(&graph).unwrap();

        for edge in graph.edges() {
            let source_index = order
                .iter()
                .position(|vertex| *vertex == edge.source)
                .unwrap();
            let destination_index = order
                .iter()
                .position(|vertex| *vertex == edge.destination)
                .unwrap();

            assert!(source_index < destination_index);
        }
    }

    #[test]
    fn disconnected_vertices_are_included_in_vertex_order() {
        let mut graph = Graph::<(), &str, ()>::new(());
        let a = graph.add_vertex("a");
        let b = graph.add_vertex("b");
        let c = graph.add_vertex("c");
        graph.add_edge(a, b, ()).unwrap();

        assert_eq!(depth_first_sort(&graph).unwrap(), vec![a, b, c]);
    }

    #[test]
    fn self_loop_is_reported_as_cycle() {
        let mut graph = Graph::<(), &str, ()>::new(());
        let a = graph.add_vertex("a");
        graph.add_edge(a, a, ()).unwrap();

        assert_eq!(
            depth_first_sort(&graph),
            Err(GraphDfsError::Cycle { vertex: a })
        );
        assert!(!is_acyclic(&graph));
    }

    #[test]
    fn incoming_dfs_detects_cycle_not_found_by_first_outgoing_walk() {
        let mut graph = Graph::<(), &str, ()>::new(());
        let a = graph.add_vertex("a");
        let b = graph.add_vertex("b");
        let c = graph.add_vertex("c");
        let d = graph.add_vertex("d");
        graph.add_edge(a, b, ()).unwrap();
        graph.add_edge(b, c, ()).unwrap();
        graph.add_edge(c, b, ()).unwrap();
        graph.add_edge(c, d, ()).unwrap();

        assert_eq!(
            depth_first_sort(&graph),
            Err(GraphDfsError::Cycle { vertex: b })
        );
    }

    #[test]
    fn acyclic_predicate_accepts_empty_and_singleton_graphs() {
        let empty = Graph::<(), (), ()>::new(());
        assert!(is_acyclic(&empty));

        let mut singleton = Graph::<(), (), ()>::new(());
        singleton.add_vertex(());
        assert!(is_acyclic(&singleton));
    }
}
