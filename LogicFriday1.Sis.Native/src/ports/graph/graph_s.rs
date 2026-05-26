//! Fixed-slot graph wrapper for the SIS static graph package.
//!
//! `graph_s.c` layers fixed-size graph, vertex, and edge slot arrays on top of
//! the base SIS graph package. This Rust port keeps the same ownership shape
//! without exposing raw pointers: each graph owns graph-level slots, every
//! vertex owns vertex slots, and every edge owns edge slots.

#[cfg(test)]
#[path = "graph.rs"]
mod graph;

#[cfg(test)]
use graph::{EdgeId, Graph, GraphError, VertexId};

#[cfg(not(test))]
use super::graph::{EdgeId, Graph, GraphError, VertexId};

use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticGraph<S = usize> {
    graph: Graph<StaticGraphData<S>, StaticVertexData<S>, StaticEdgeData<S>>,
}

impl<S> StaticGraph<S> {
    pub fn new(graph_slots: usize, vertex_slots: usize, edge_slots: usize) -> Self {
        Self {
            graph: Graph::new(StaticGraphData {
                graph_slots: empty_slots(graph_slots),
                vertex_slot_count: vertex_slots,
                edge_slot_count: edge_slots,
            }),
        }
    }

    pub fn graph_slot_count(&self) -> usize {
        self.graph.data.graph_slots.len()
    }

    pub fn vertex_slot_count(&self) -> usize {
        self.graph.data.vertex_slot_count
    }

    pub fn edge_slot_count(&self) -> usize {
        self.graph.data.edge_slot_count
    }

    pub fn vertex_count(&self) -> usize {
        self.graph.vertex_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn graph(&self) -> &Graph<StaticGraphData<S>, StaticVertexData<S>, StaticEdgeData<S>> {
        &self.graph
    }

    pub fn graph_mut(
        &mut self,
    ) -> &mut Graph<StaticGraphData<S>, StaticVertexData<S>, StaticEdgeData<S>> {
        &mut self.graph
    }

    pub fn vertices(&self) -> impl Iterator<Item = VertexId> + '_ {
        self.graph.vertices().map(|vertex| vertex.id)
    }

    pub fn edges(&self) -> impl Iterator<Item = EdgeId> + '_ {
        self.graph.edges().map(|edge| edge.id)
    }

    pub fn add_vertex(&mut self) -> VertexId {
        self.graph.add_vertex(StaticVertexData {
            slots: empty_slots(self.graph.data.vertex_slot_count),
        })
    }

    pub fn add_edge(
        &mut self,
        source: VertexId,
        destination: VertexId,
    ) -> Result<EdgeId, StaticGraphError> {
        Ok(self.graph.add_edge(
            source,
            destination,
            StaticEdgeData {
                slots: empty_slots(self.graph.data.edge_slot_count),
            },
        )?)
    }

    pub fn delete_edge<F>(
        &mut self,
        edge: EdgeId,
        mut free_edge_slots: Option<F>,
    ) -> Result<StaticEdgeData<S>, StaticGraphError>
    where
        F: FnMut(Vec<Option<S>>),
    {
        let edge_data = self.graph.delete_edge(edge)?;

        if let Some(free_edge_slots) = free_edge_slots.as_mut() {
            free_edge_slots(edge_data.slots);
            Ok(StaticEdgeData { slots: Vec::new() })
        } else {
            Ok(edge_data)
        }
    }

    pub fn delete_vertex<FV, FE>(
        &mut self,
        vertex: VertexId,
        mut free_vertex_slots: Option<FV>,
        mut free_edge_slots: Option<FE>,
    ) -> Result<DeletedStaticVertex<S>, StaticGraphError>
    where
        FV: FnMut(Vec<Option<S>>),
        FE: FnMut(Vec<Option<S>>),
    {
        let deleted = self.graph.delete_vertex(vertex)?;
        let vertex_slots = deleted.vertex_data.slots;
        let edge_slots = deleted
            .edge_data
            .into_iter()
            .map(|edge_data| edge_data.slots)
            .collect::<Vec<_>>();

        if let Some(free_edge_slots) = free_edge_slots.as_mut() {
            for slots in edge_slots {
                free_edge_slots(slots);
            }

            if let Some(free_vertex_slots) = free_vertex_slots.as_mut() {
                free_vertex_slots(vertex_slots);
            }

            return Ok(DeletedStaticVertex {
                vertex_slots: Vec::new(),
                edge_slots: Vec::new(),
            });
        }

        if let Some(free_vertex_slots) = free_vertex_slots.as_mut() {
            free_vertex_slots(vertex_slots);
            Ok(DeletedStaticVertex {
                vertex_slots: Vec::new(),
                edge_slots,
            })
        } else {
            Ok(DeletedStaticVertex {
                vertex_slots,
                edge_slots,
            })
        }
    }

    pub fn set_graph_slot(&mut self, index: usize, value: S) -> Result<(), StaticGraphError> {
        set_slot(
            &mut self.graph.data.graph_slots,
            index,
            value,
            SlotOwner::Graph,
        )
    }

    pub fn get_graph_slot(&self, index: usize) -> Result<Option<&S>, StaticGraphError> {
        get_slot(&self.graph.data.graph_slots, index, SlotOwner::Graph)
    }

    pub fn clear_graph_slot(&mut self, index: usize) -> Result<Option<S>, StaticGraphError> {
        clear_slot(&mut self.graph.data.graph_slots, index, SlotOwner::Graph)
    }

    pub fn set_vertex_slot(
        &mut self,
        vertex: VertexId,
        index: usize,
        value: S,
    ) -> Result<(), StaticGraphError> {
        let vertex = self.graph.vertex_mut(vertex)?;
        set_slot(&mut vertex.data.slots, index, value, SlotOwner::Vertex)
    }

    pub fn get_vertex_slot(
        &self,
        vertex: VertexId,
        index: usize,
    ) -> Result<Option<&S>, StaticGraphError> {
        let vertex = self.graph.vertex(vertex)?;
        get_slot(&vertex.data.slots, index, SlotOwner::Vertex)
    }

    pub fn clear_vertex_slot(
        &mut self,
        vertex: VertexId,
        index: usize,
    ) -> Result<Option<S>, StaticGraphError> {
        let vertex = self.graph.vertex_mut(vertex)?;
        clear_slot(&mut vertex.data.slots, index, SlotOwner::Vertex)
    }

    pub fn set_edge_slot(
        &mut self,
        edge: EdgeId,
        index: usize,
        value: S,
    ) -> Result<(), StaticGraphError> {
        let edge = self.graph.edge_mut(edge)?;
        set_slot(&mut edge.data.slots, index, value, SlotOwner::Edge)
    }

    pub fn get_edge_slot(
        &self,
        edge: EdgeId,
        index: usize,
    ) -> Result<Option<&S>, StaticGraphError> {
        let edge = self.graph.edge(edge)?;
        get_slot(&edge.data.slots, index, SlotOwner::Edge)
    }

    pub fn clear_edge_slot(
        &mut self,
        edge: EdgeId,
        index: usize,
    ) -> Result<Option<S>, StaticGraphError> {
        let edge = self.graph.edge_mut(edge)?;
        clear_slot(&mut edge.data.slots, index, SlotOwner::Edge)
    }
}

impl<S: Clone> StaticGraph<S> {
    pub fn duplicate<FG, FV, FE>(
        &self,
        mut copy_graph_slots: Option<FG>,
        mut copy_vertex_slots: Option<FV>,
        mut copy_edge_slots: Option<FE>,
    ) -> Result<Self, StaticGraphError>
    where
        FG: FnMut(&[Option<S>]) -> Vec<Option<S>>,
        FV: FnMut(&[Option<S>]) -> Vec<Option<S>>,
        FE: FnMut(&[Option<S>]) -> Vec<Option<S>>,
    {
        let graph = self.graph.duplicate(
            |data| StaticGraphData {
                graph_slots: copy_slots(&data.graph_slots, copy_graph_slots.as_mut()),
                vertex_slot_count: data.vertex_slot_count,
                edge_slot_count: data.edge_slot_count,
            },
            |data| StaticVertexData {
                slots: copy_slots(&data.slots, copy_vertex_slots.as_mut()),
            },
            |data| StaticEdgeData {
                slots: copy_slots(&data.slots, copy_edge_slots.as_mut()),
            },
        )?;

        Ok(Self { graph })
    }

    pub fn copy_graph_slots_from<FG>(
        &mut self,
        source: &Self,
        mut copy_graph_slots: Option<FG>,
    ) -> Result<(), StaticGraphError>
    where
        FG: FnMut(&[Option<S>]) -> Vec<Option<S>>,
    {
        if self.graph_slot_count() != source.graph_slot_count() {
            return Err(StaticGraphError::SlotCountMismatch {
                owner: SlotOwner::Graph,
                left: source.graph_slot_count(),
                right: self.graph_slot_count(),
            });
        }

        self.graph.data.graph_slots =
            copy_slots(&source.graph.data.graph_slots, copy_graph_slots.as_mut());
        Ok(())
    }

    pub fn copy_vertex_slots<FV>(
        &mut self,
        source: VertexId,
        destination: VertexId,
        mut copy_vertex_slots: Option<FV>,
    ) -> Result<(), StaticGraphError>
    where
        FV: FnMut(&[Option<S>]) -> Vec<Option<S>>,
    {
        let source_slots = self.graph.vertex(source)?.data.slots.clone();
        let destination_slots = &mut self.graph.vertex_mut(destination)?.data.slots;
        copy_into_slots(
            &source_slots,
            destination_slots,
            SlotOwner::Vertex,
            copy_vertex_slots.as_mut(),
        )
    }

    pub fn copy_edge_slots<FE>(
        &mut self,
        source: EdgeId,
        destination: EdgeId,
        mut copy_edge_slots: Option<FE>,
    ) -> Result<(), StaticGraphError>
    where
        FE: FnMut(&[Option<S>]) -> Vec<Option<S>>,
    {
        let source_slots = self.graph.edge(source)?.data.slots.clone();
        let destination_slots = &mut self.graph.edge_mut(destination)?.data.slots;
        copy_into_slots(
            &source_slots,
            destination_slots,
            SlotOwner::Edge,
            copy_edge_slots.as_mut(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticGraphData<S> {
    pub graph_slots: Vec<Option<S>>,
    pub vertex_slot_count: usize,
    pub edge_slot_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticVertexData<S> {
    pub slots: Vec<Option<S>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticEdgeData<S> {
    pub slots: Vec<Option<S>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeletedStaticVertex<S> {
    pub vertex_slots: Vec<Option<S>>,
    pub edge_slots: Vec<Vec<Option<S>>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotOwner {
    Graph,
    Vertex,
    Edge,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StaticGraphError {
    Graph(GraphError),
    SlotOutOfRange {
        owner: SlotOwner,
        index: usize,
        len: usize,
    },
    SlotCountMismatch {
        owner: SlotOwner,
        left: usize,
        right: usize,
    },
    CopyProducedWrongSlotCount {
        owner: SlotOwner,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for StaticGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Graph(error) => write!(f, "{error}"),
            Self::SlotOutOfRange { owner, index, len } => write!(
                f,
                "{owner:?} static graph slot index {index} is outside slot count {len}"
            ),
            Self::SlotCountMismatch { owner, left, right } => write!(
                f,
                "{owner:?} static graph slot counts differ: {left} versus {right}"
            ),
            Self::CopyProducedWrongSlotCount {
                owner,
                expected,
                actual,
            } => write!(
                f,
                "{owner:?} static graph slot copy returned {actual} slots; expected {expected}"
            ),
        }
    }
}

impl Error for StaticGraphError {}

impl From<GraphError> for StaticGraphError {
    fn from(value: GraphError) -> Self {
        Self::Graph(value)
    }
}

fn empty_slots<S>(count: usize) -> Vec<Option<S>> {
    std::iter::repeat_with(|| None).take(count).collect()
}

fn set_slot<S>(
    slots: &mut [Option<S>],
    index: usize,
    value: S,
    owner: SlotOwner,
) -> Result<(), StaticGraphError> {
    let len = slots.len();
    let slot = slots
        .get_mut(index)
        .ok_or(StaticGraphError::SlotOutOfRange { owner, index, len })?;
    *slot = Some(value);
    Ok(())
}

fn get_slot<S>(
    slots: &[Option<S>],
    index: usize,
    owner: SlotOwner,
) -> Result<Option<&S>, StaticGraphError> {
    let len = slots.len();
    slots
        .get(index)
        .map(Option::as_ref)
        .ok_or(StaticGraphError::SlotOutOfRange { owner, index, len })
}

fn clear_slot<S>(
    slots: &mut [Option<S>],
    index: usize,
    owner: SlotOwner,
) -> Result<Option<S>, StaticGraphError> {
    let len = slots.len();
    let slot = slots
        .get_mut(index)
        .ok_or(StaticGraphError::SlotOutOfRange { owner, index, len })?;
    Ok(slot.take())
}

fn copy_slots<S: Clone, F>(source: &[Option<S>], copier: Option<&mut F>) -> Vec<Option<S>>
where
    F: FnMut(&[Option<S>]) -> Vec<Option<S>>,
{
    if let Some(copier) = copier {
        copier(source)
    } else {
        source.to_vec()
    }
}

fn copy_into_slots<S: Clone, F>(
    source: &[Option<S>],
    destination: &mut Vec<Option<S>>,
    owner: SlotOwner,
    copier: Option<&mut F>,
) -> Result<(), StaticGraphError>
where
    F: FnMut(&[Option<S>]) -> Vec<Option<S>>,
{
    if source.len() != destination.len() {
        return Err(StaticGraphError::SlotCountMismatch {
            owner,
            left: source.len(),
            right: destination.len(),
        });
    }

    let copied = copy_slots(source, copier);
    if copied.len() != destination.len() {
        return Err(StaticGraphError::CopyProducedWrongSlotCount {
            owner,
            expected: destination.len(),
            actual: copied.len(),
        });
    }

    *destination = copied;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn allocates_graph_vertex_and_edge_slots() {
        let mut graph = StaticGraph::<String>::new(2, 3, 1);
        let a = graph.add_vertex();
        let b = graph.add_vertex();
        let edge = graph.add_edge(a, b).unwrap();

        assert_eq!(graph.graph_slot_count(), 2);
        assert_eq!(graph.vertex_slot_count(), 3);
        assert_eq!(graph.edge_slot_count(), 1);
        assert_eq!(graph.vertex_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert_eq!(
            graph.graph().vertex(a).unwrap().data.slots,
            vec![None, None, None]
        );
        assert_eq!(graph.graph().edge(edge).unwrap().data.slots, vec![None]);
    }

    #[test]
    fn graph_vertex_and_edge_slots_round_trip_values() {
        let mut graph = StaticGraph::<String>::new(1, 1, 1);
        let a = graph.add_vertex();
        let b = graph.add_vertex();
        let edge = graph.add_edge(a, b).unwrap();

        graph.set_graph_slot(0, "graph".to_string()).unwrap();
        graph.set_vertex_slot(a, 0, "vertex".to_string()).unwrap();
        graph.set_edge_slot(edge, 0, "edge".to_string()).unwrap();

        assert_eq!(graph.get_graph_slot(0).unwrap(), Some(&"graph".to_string()));
        assert_eq!(
            graph.get_vertex_slot(a, 0).unwrap(),
            Some(&"vertex".to_string())
        );
        assert_eq!(
            graph.get_edge_slot(edge, 0).unwrap(),
            Some(&"edge".to_string())
        );
        assert_eq!(
            graph.clear_vertex_slot(a, 0).unwrap(),
            Some("vertex".to_string())
        );
        assert_eq!(graph.get_vertex_slot(a, 0).unwrap(), None);
    }

    #[test]
    fn rejects_unknown_graph_objects_and_bad_slot_indexes() {
        let mut graph = StaticGraph::<i32>::new(1, 1, 1);
        let a = graph.add_vertex();

        assert_eq!(
            graph.set_graph_slot(2, 7),
            Err(StaticGraphError::SlotOutOfRange {
                owner: SlotOwner::Graph,
                index: 2,
                len: 1
            })
        );
        assert_eq!(
            graph.get_vertex_slot(VertexId(99), 0),
            Err(StaticGraphError::Graph(GraphError::UnknownVertex(
                VertexId(99)
            )))
        );
        assert_eq!(
            graph.add_edge(a, VertexId(99)),
            Err(StaticGraphError::Graph(GraphError::UnknownVertex(
                VertexId(99)
            )))
        );
    }

    #[test]
    fn duplicate_defaults_to_shallow_slot_clone_and_rebuilds_edges() {
        let mut graph = StaticGraph::<String>::new(1, 1, 1);
        let a = graph.add_vertex();
        let b = graph.add_vertex();
        let edge = graph.add_edge(a, b).unwrap();
        graph.set_graph_slot(0, "g".to_string()).unwrap();
        graph.set_vertex_slot(a, 0, "a".to_string()).unwrap();
        graph.set_edge_slot(edge, 0, "e".to_string()).unwrap();

        let duplicate = graph
            .duplicate(
                None::<fn(&[Option<String>]) -> Vec<Option<String>>>,
                None::<fn(&[Option<String>]) -> Vec<Option<String>>>,
                None::<fn(&[Option<String>]) -> Vec<Option<String>>>,
            )
            .unwrap();

        assert_eq!(duplicate.get_graph_slot(0).unwrap(), Some(&"g".to_string()));
        assert_eq!(
            duplicate
                .graph()
                .vertices()
                .map(|vertex| vertex.data.slots.clone())
                .collect::<Vec<_>>(),
            vec![vec![Some("a".to_string())], vec![None]]
        );
        let duplicated_edge = duplicate.graph().edges().next().unwrap();
        assert_eq!(duplicated_edge.data.slots, vec![Some("e".to_string())]);
        assert!(duplicate.graph().check().unwrap().warnings.is_empty());
    }

    #[test]
    fn duplicate_uses_custom_copy_hooks() {
        let mut graph = StaticGraph::<i32>::new(1, 1, 1);
        let a = graph.add_vertex();
        let b = graph.add_vertex();
        let edge = graph.add_edge(a, b).unwrap();
        graph.set_graph_slot(0, 1).unwrap();
        graph.set_vertex_slot(a, 0, 2).unwrap();
        graph.set_edge_slot(edge, 0, 3).unwrap();

        let duplicate = graph
            .duplicate(
                Some(|slots: &[Option<i32>]| {
                    slots
                        .iter()
                        .map(|slot| slot.map(|value| value + 10))
                        .collect()
                }),
                Some(|slots: &[Option<i32>]| {
                    slots
                        .iter()
                        .map(|slot| slot.map(|value| value + 20))
                        .collect()
                }),
                Some(|slots: &[Option<i32>]| {
                    slots
                        .iter()
                        .map(|slot| slot.map(|value| value + 30))
                        .collect()
                }),
            )
            .unwrap();

        assert_eq!(duplicate.get_graph_slot(0).unwrap(), Some(&11));
        assert_eq!(
            duplicate.graph().vertices().next().unwrap().data.slots,
            vec![Some(22)]
        );
        assert_eq!(
            duplicate.graph().edges().next().unwrap().data.slots,
            vec![Some(33)]
        );
    }

    #[test]
    fn copy_graph_vertex_and_edge_slots_between_existing_objects() {
        let mut source = StaticGraph::<i32>::new(2, 1, 1);
        let source_a = source.add_vertex();
        let source_b = source.add_vertex();
        let source_edge = source.add_edge(source_a, source_b).unwrap();
        source.set_graph_slot(0, 10).unwrap();
        source.set_graph_slot(1, 11).unwrap();

        let mut destination = StaticGraph::<i32>::new(2, 1, 1);
        let destination_a = destination.add_vertex();
        let destination_b = destination.add_vertex();
        let destination_edge = destination.add_edge(destination_a, destination_b).unwrap();
        destination.set_vertex_slot(destination_a, 0, 1).unwrap();
        destination.set_edge_slot(destination_edge, 0, 2).unwrap();

        destination
            .copy_graph_slots_from(&source, None::<fn(&[Option<i32>]) -> Vec<Option<i32>>>)
            .unwrap();
        destination.set_vertex_slot(destination_b, 0, 20).unwrap();
        destination
            .copy_vertex_slots(
                destination_b,
                destination_a,
                None::<fn(&[Option<i32>]) -> Vec<Option<i32>>>,
            )
            .unwrap();
        destination.set_edge_slot(destination_edge, 0, 30).unwrap();
        destination
            .copy_edge_slots(
                destination_edge,
                destination_edge,
                Some(|slots: &[Option<i32>]| {
                    slots
                        .iter()
                        .map(|slot| slot.map(|value| value + 1))
                        .collect()
                }),
            )
            .unwrap();

        assert_eq!(destination.get_graph_slot(0).unwrap(), Some(&10));
        assert_eq!(destination.get_graph_slot(1).unwrap(), Some(&11));
        assert_eq!(
            destination.get_vertex_slot(destination_a, 0).unwrap(),
            Some(&20)
        );
        assert_eq!(
            destination.get_edge_slot(destination_edge, 0).unwrap(),
            Some(&31)
        );
        assert_eq!(source.get_edge_slot(source_edge, 0).unwrap(), None);
    }

    #[test]
    fn copy_rejects_different_slot_counts() {
        let source = StaticGraph::<i32>::new(2, 1, 1);
        let mut destination = StaticGraph::<i32>::new(1, 1, 1);

        assert_eq!(
            destination
                .copy_graph_slots_from(&source, None::<fn(&[Option<i32>]) -> Vec<Option<i32>>>),
            Err(StaticGraphError::SlotCountMismatch {
                owner: SlotOwner::Graph,
                left: 2,
                right: 1
            })
        );
    }

    #[test]
    fn delete_edge_and_vertex_return_owned_slots_or_call_hooks() {
        let mut graph = StaticGraph::<i32>::new(0, 1, 1);
        let a = graph.add_vertex();
        let b = graph.add_vertex();
        let edge = graph.add_edge(a, b).unwrap();
        graph.set_vertex_slot(a, 0, 7).unwrap();
        graph.set_edge_slot(edge, 0, 8).unwrap();

        let deleted_edge = graph
            .delete_edge(edge, None::<fn(Vec<Option<i32>>)>)
            .unwrap();
        assert_eq!(deleted_edge.slots, vec![Some(8)]);
        assert_eq!(graph.edge_count(), 0);

        let edge = graph.add_edge(a, b).unwrap();
        graph.set_edge_slot(edge, 0, 9).unwrap();
        let freed = Rc::new(RefCell::new(Vec::new()));
        let freed_edges = Rc::clone(&freed);
        let deleted_vertex = graph
            .delete_vertex(
                a,
                Some(|slots| freed.borrow_mut().push(slots)),
                Some(|slots| freed_edges.borrow_mut().push(slots)),
            )
            .unwrap();

        assert_eq!(deleted_vertex.vertex_slots, Vec::<Option<i32>>::new());
        assert_eq!(deleted_vertex.edge_slots, Vec::<Vec<Option<i32>>>::new());
        assert_eq!(&*freed.borrow(), &vec![vec![Some(9)], vec![Some(7)]]);
    }
}
