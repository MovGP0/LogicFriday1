//! Native Rust graph substrate for the SIS graph package.
//!
//! The original implementation owns ordered vertex and edge lists and keeps
//! each vertex's incoming and outgoing edge lists in sync with the graph-wide
//! edge list. This port keeps those invariants with stable IDs and safe Rust
//! containers so later timing and network ports can share one graph model.

use std::cmp::Ordering;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

static NEXT_UNIQUE_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EdgeId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vertex<V> {
    pub id: VertexId,
    pub unique_id: usize,
    pub data: V,
    incoming: Vec<EdgeId>,
    outgoing: Vec<EdgeId>,
}

impl<V> Vertex<V> {
    pub fn incoming_edges(&self) -> &[EdgeId] {
        &self.incoming
    }

    pub fn outgoing_edges(&self) -> &[EdgeId] {
        &self.outgoing
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge<E> {
    pub id: EdgeId,
    pub unique_id: usize,
    pub source: VertexId,
    pub destination: VertexId,
    pub data: E,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Graph<G = (), V = (), E = ()> {
    pub data: G,
    vertices: Vec<Option<Vertex<V>>>,
    edges: Vec<Option<Edge<E>>>,
    vertex_order: Vec<VertexId>,
    edge_order: Vec<EdgeId>,
}

impl<G, V, E> Graph<G, V, E> {
    pub fn new(data: G) -> Self {
        Self {
            data,
            vertices: Vec::new(),
            edges: Vec::new(),
            vertex_order: Vec::new(),
            edge_order: Vec::new(),
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.vertex_order.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edge_order.len()
    }

    pub fn vertices(&self) -> impl Iterator<Item = &Vertex<V>> {
        self.vertex_order
            .iter()
            .filter_map(|id| self.vertex_slot(*id).and_then(Option::as_ref))
    }

    pub fn edges(&self) -> impl Iterator<Item = &Edge<E>> {
        self.edge_order
            .iter()
            .filter_map(|id| self.edge_slot(*id).and_then(Option::as_ref))
    }

    pub fn vertex(&self, id: VertexId) -> Result<&Vertex<V>, GraphError> {
        self.vertex_slot(id)
            .and_then(Option::as_ref)
            .ok_or(GraphError::UnknownVertex(id))
    }

    pub fn vertex_mut(&mut self, id: VertexId) -> Result<&mut Vertex<V>, GraphError> {
        self.vertex_slot_mut(id)
            .and_then(Option::as_mut)
            .ok_or(GraphError::UnknownVertex(id))
    }

    pub fn edge(&self, id: EdgeId) -> Result<&Edge<E>, GraphError> {
        self.edge_slot(id)
            .and_then(Option::as_ref)
            .ok_or(GraphError::UnknownEdge(id))
    }

    pub fn edge_mut(&mut self, id: EdgeId) -> Result<&mut Edge<E>, GraphError> {
        self.edge_slot_mut(id)
            .and_then(Option::as_mut)
            .ok_or(GraphError::UnknownEdge(id))
    }

    pub fn add_vertex(&mut self, data: V) -> VertexId {
        let id = VertexId(self.vertices.len());
        let vertex = Vertex {
            id,
            unique_id: next_unique_id(),
            data,
            incoming: Vec::new(),
            outgoing: Vec::new(),
        };

        self.vertices.push(Some(vertex));
        self.vertex_order.push(id);
        id
    }

    pub fn add_edge(
        &mut self,
        source: VertexId,
        destination: VertexId,
        data: E,
    ) -> Result<EdgeId, GraphError> {
        self.validate_vertex(source)?;
        self.validate_vertex(destination)?;

        let id = EdgeId(self.edges.len());
        let edge = Edge {
            id,
            unique_id: next_unique_id(),
            source,
            destination,
            data,
        };

        self.edges.push(Some(edge));
        self.edge_order.push(id);
        self.vertex_mut(source)?.outgoing.push(id);
        self.vertex_mut(destination)?.incoming.push(id);

        Ok(id)
    }

    pub fn delete_edge(&mut self, id: EdgeId) -> Result<E, GraphError> {
        let edge = self
            .edge_slot_mut(id)
            .and_then(Option::take)
            .ok_or(GraphError::UnknownEdge(id))?;

        remove_id(&mut self.vertex_mut(edge.source)?.outgoing, id)?;
        remove_id(&mut self.vertex_mut(edge.destination)?.incoming, id)?;
        remove_id(&mut self.edge_order, id)?;

        Ok(edge.data)
    }

    pub fn delete_vertex(&mut self, id: VertexId) -> Result<DeletedVertex<V, E>, GraphError> {
        self.validate_vertex(id)?;

        let incident_edges = {
            let vertex = self.vertex(id)?;
            let mut edges = Vec::with_capacity(vertex.incoming.len() + vertex.outgoing.len());
            edges.extend(vertex.incoming.iter().copied());
            edges.extend(vertex.outgoing.iter().copied());
            edges
        };

        let mut seen = HashSet::new();
        let mut deleted_edges = Vec::new();
        for edge_id in incident_edges {
            if seen.insert(edge_id) {
                deleted_edges.push(self.delete_edge(edge_id)?);
            }
        }

        let vertex = self
            .vertex_slot_mut(id)
            .and_then(Option::take)
            .ok_or(GraphError::UnknownVertex(id))?;
        remove_id(&mut self.vertex_order, id)?;

        Ok(DeletedVertex {
            vertex_data: vertex.data,
            edge_data: deleted_edges,
        })
    }

    pub fn check(&self) -> Result<GraphCheck, GraphError> {
        let mut warnings = Vec::new();

        for edge in self.edges() {
            self.validate_vertex(edge.source)?;
            self.validate_vertex(edge.destination)?;

            let source = self.vertex(edge.source)?;
            if !source.outgoing.contains(&edge.id) {
                return Err(GraphError::MissingSourceBackReference(edge.id));
            }

            let destination = self.vertex(edge.destination)?;
            if !destination.incoming.contains(&edge.id) {
                return Err(GraphError::MissingDestinationBackReference(edge.id));
            }
        }

        for vertex in self.vertices() {
            if vertex.incoming.is_empty() && vertex.outgoing.is_empty() {
                warnings.push(GraphCheckWarning::UnconnectedVertex(vertex.id));
            }

            for edge_id in &vertex.incoming {
                let edge = self.edge(*edge_id)?;
                if edge.destination != vertex.id {
                    return Err(GraphError::IncorrectDestinationBackReference {
                        vertex: vertex.id,
                        edge: *edge_id,
                    });
                }
            }

            for edge_id in &vertex.outgoing {
                let edge = self.edge(*edge_id)?;
                if edge.source != vertex.id {
                    return Err(GraphError::IncorrectSourceBackReference {
                        vertex: vertex.id,
                        edge: *edge_id,
                    });
                }
            }
        }

        Ok(GraphCheck { warnings })
    }

    pub fn duplicate<G2, V2, E2, FG, FV, FE>(
        &self,
        copy_graph: FG,
        mut copy_vertex: FV,
        mut copy_edge: FE,
    ) -> Result<Graph<G2, V2, E2>, GraphError>
    where
        FG: FnOnce(&G) -> G2,
        FV: FnMut(&V) -> V2,
        FE: FnMut(&E) -> E2,
    {
        let mut duplicate = Graph::new(copy_graph(&self.data));
        let mut vertex_map = Vec::with_capacity(self.vertices.len());
        vertex_map.resize(self.vertices.len(), None);

        for vertex in self.vertices() {
            let new_id = duplicate.add_vertex(copy_vertex(&vertex.data));
            vertex_map[vertex.id.0] = Some(new_id);
        }

        for edge in self.edges() {
            let source = vertex_map
                .get(edge.source.0)
                .and_then(|id| *id)
                .ok_or(GraphError::UnknownVertex(edge.source))?;
            let destination = vertex_map
                .get(edge.destination.0)
                .and_then(|id| *id)
                .ok_or(GraphError::UnknownVertex(edge.destination))?;
            duplicate.add_edge(source, destination, copy_edge(&edge.data))?;
        }

        Ok(duplicate)
    }

    pub fn sorted_vertices<F>(&self, mut compare: F) -> Vec<VertexId>
    where
        F: FnMut(&Vertex<V>, &Vertex<V>) -> Ordering,
    {
        let mut vertices = self.vertices().collect::<Vec<_>>();
        vertices.sort_by(|left, right| compare(left, right));
        vertices.into_iter().map(|vertex| vertex.id).collect()
    }

    fn validate_vertex(&self, id: VertexId) -> Result<(), GraphError> {
        self.vertex(id).map(|_| ())
    }

    fn vertex_slot(&self, id: VertexId) -> Option<&Option<Vertex<V>>> {
        self.vertices.get(id.0)
    }

    fn vertex_slot_mut(&mut self, id: VertexId) -> Option<&mut Option<Vertex<V>>> {
        self.vertices.get_mut(id.0)
    }

    fn edge_slot(&self, id: EdgeId) -> Option<&Option<Edge<E>>> {
        self.edges.get(id.0)
    }

    fn edge_slot_mut(&mut self, id: EdgeId) -> Option<&mut Option<Edge<E>>> {
        self.edges.get_mut(id.0)
    }
}

impl<G: Clone, V: Clone, E: Clone> Graph<G, V, E> {
    pub fn duplicate_cloned(&self) -> Result<Self, GraphError> {
        self.duplicate(Clone::clone, Clone::clone, Clone::clone)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeletedVertex<V, E> {
    pub vertex_data: V,
    pub edge_data: Vec<E>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphCheck {
    pub warnings: Vec<GraphCheckWarning>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphCheckWarning {
    UnconnectedVertex(VertexId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphError {
    UnknownVertex(VertexId),
    UnknownEdge(EdgeId),
    MissingSourceBackReference(EdgeId),
    MissingDestinationBackReference(EdgeId),
    IncorrectSourceBackReference { vertex: VertexId, edge: EdgeId },
    IncorrectDestinationBackReference { vertex: VertexId, edge: EdgeId },
    ListItemMissing,
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownVertex(id) => write!(f, "unknown graph vertex {:?}", id),
            Self::UnknownEdge(id) => write!(f, "unknown graph edge {:?}", id),
            Self::MissingSourceBackReference(id) => {
                write!(f, "edge {:?} is missing from its source vertex", id)
            }
            Self::MissingDestinationBackReference(id) => {
                write!(f, "edge {:?} is missing from its destination vertex", id)
            }
            Self::IncorrectSourceBackReference { vertex, edge } => {
                write!(
                    f,
                    "outgoing edge {:?} does not point back to vertex {:?}",
                    edge, vertex
                )
            }
            Self::IncorrectDestinationBackReference { vertex, edge } => {
                write!(
                    f,
                    "incoming edge {:?} does not point back to vertex {:?}",
                    edge, vertex
                )
            }
            Self::ListItemMissing => write!(f, "graph list item was missing"),
        }
    }
}

impl Error for GraphError {}

fn next_unique_id() -> usize {
    NEXT_UNIQUE_ID.fetch_add(1, AtomicOrdering::Relaxed)
}

fn remove_id<T>(items: &mut Vec<T>, id: T) -> Result<(), GraphError>
where
    T: Copy + Eq,
{
    let index = items
        .iter()
        .position(|candidate| *candidate == id)
        .ok_or(GraphError::ListItemMissing)?;
    items.remove(index);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_vertex_and_edge_preserves_order_and_back_references() {
        let mut graph = Graph::<String, i32, &'static str>::new("graph".to_string());
        let a = graph.add_vertex(10);
        let b = graph.add_vertex(20);
        let edge = graph.add_edge(a, b, "a-b").unwrap();

        assert_eq!(graph.vertex_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert_eq!(
            graph.vertices().map(|vertex| vertex.id).collect::<Vec<_>>(),
            vec![a, b]
        );
        assert_eq!(
            graph.edges().map(|edge| edge.id).collect::<Vec<_>>(),
            vec![edge]
        );
        assert_eq!(graph.vertex(a).unwrap().outgoing_edges(), &[edge]);
        assert_eq!(graph.vertex(b).unwrap().incoming_edges(), &[edge]);
        assert!(graph.check().unwrap().warnings.is_empty());
    }

    #[test]
    fn adding_edge_rejects_missing_vertices() {
        let mut graph = Graph::<(), (), ()>::new(());
        let a = graph.add_vertex(());

        assert_eq!(
            graph.add_edge(a, VertexId(99), ()),
            Err(GraphError::UnknownVertex(VertexId(99)))
        );
    }

    #[test]
    fn delete_edge_removes_graph_and_vertex_references() {
        let mut graph = Graph::<(), &str, i32>::new(());
        let a = graph.add_vertex("a");
        let b = graph.add_vertex("b");
        let edge = graph.add_edge(a, b, 42).unwrap();

        assert_eq!(graph.delete_edge(edge).unwrap(), 42);

        assert_eq!(graph.edge_count(), 0);
        assert!(graph.vertex(a).unwrap().outgoing_edges().is_empty());
        assert!(graph.vertex(b).unwrap().incoming_edges().is_empty());
        assert_eq!(graph.edge(edge), Err(GraphError::UnknownEdge(edge)));
    }

    #[test]
    fn delete_vertex_removes_all_incident_edges_once() {
        let mut graph = Graph::<(), &str, &str>::new(());
        let a = graph.add_vertex("a");
        let b = graph.add_vertex("b");
        let c = graph.add_vertex("c");
        let self_edge = graph.add_edge(b, b, "bb").unwrap();
        let in_edge = graph.add_edge(a, b, "ab").unwrap();
        let out_edge = graph.add_edge(b, c, "bc").unwrap();

        let deleted = graph.delete_vertex(b).unwrap();

        assert_eq!(deleted.vertex_data, "b");
        assert_eq!(deleted.edge_data, vec!["bb", "ab", "bc"]);
        assert_eq!(graph.vertex_count(), 2);
        assert_eq!(graph.edge_count(), 0);
        assert_eq!(
            graph.edge(self_edge),
            Err(GraphError::UnknownEdge(self_edge))
        );
        assert_eq!(graph.edge(in_edge), Err(GraphError::UnknownEdge(in_edge)));
        assert_eq!(graph.edge(out_edge), Err(GraphError::UnknownEdge(out_edge)));
        assert!(graph.vertex(a).unwrap().outgoing_edges().is_empty());
        assert!(graph.vertex(c).unwrap().incoming_edges().is_empty());
    }

    #[test]
    fn duplicate_clones_payloads_and_rebuilds_edge_endpoints() {
        let mut graph = Graph::<String, String, String>::new("g".to_string());
        let a = graph.add_vertex("a".to_string());
        let b = graph.add_vertex("b".to_string());
        graph.add_edge(a, b, "edge".to_string()).unwrap();

        let duplicate = graph
            .duplicate(
                |data| format!("{data}2"),
                |data| format!("{data}2"),
                |data| format!("{data}2"),
            )
            .unwrap();

        assert_eq!(duplicate.data, "g2");
        assert_eq!(
            duplicate
                .vertices()
                .map(|vertex| vertex.data.as_str())
                .collect::<Vec<_>>(),
            vec!["a2", "b2"]
        );

        let edge = duplicate.edges().next().unwrap();
        assert_eq!(edge.data, "edge2");
        assert_eq!(duplicate.vertex(edge.source).unwrap().data, "a2");
        assert_eq!(duplicate.vertex(edge.destination).unwrap().data, "b2");
    }

    #[test]
    fn check_reports_unconnected_vertices_as_warnings() {
        let mut graph = Graph::<(), &str, ()>::new(());
        let a = graph.add_vertex("a");
        let b = graph.add_vertex("b");
        graph.add_edge(a, b, ()).unwrap();
        let c = graph.add_vertex("c");

        assert_eq!(
            graph.check().unwrap(),
            GraphCheck {
                warnings: vec![GraphCheckWarning::UnconnectedVertex(c)]
            }
        );
    }

    #[test]
    fn sorted_vertices_uses_supplied_comparator() {
        let mut graph = Graph::<(), i32, ()>::new(());
        let low = graph.add_vertex(1);
        let high = graph.add_vertex(9);
        let mid = graph.add_vertex(5);

        let sorted = graph.sorted_vertices(|left, right| right.data.cmp(&left.data));

        assert_eq!(sorted, vec![high, mid, low]);
    }

    #[test]
    fn unique_ids_keep_increasing_across_graphs() {
        let mut first = Graph::<(), (), ()>::new(());
        let mut second = Graph::<(), (), ()>::new(());

        let first_vertex = first.add_vertex(());
        let second_vertex = second.add_vertex(());
        let first_id = first.vertex(first_vertex).unwrap().unique_id;
        let second_id = second.vertex(second_vertex).unwrap().unique_id;

        assert!(second_id > first_id);
    }
}
