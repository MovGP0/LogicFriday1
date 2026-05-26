//! Owned ACT reduction primitives.
//!
//! The reducer canonicalizes a reachable ACT DAG bottom-up: terminals with the
//! same value are shared, decision vertices whose low and high edges reduce to
//! the same child are bypassed, and remaining vertices at the same variable
//! index with identical reduced children are merged.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActVertexId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActGraph {
    vertices: Vec<ActVertex>,
    root: ActVertexId,
    index_size: usize,
}

impl ActGraph {
    pub fn new(index_size: usize) -> Self {
        Self {
            vertices: Vec::new(),
            root: ActVertexId(0),
            index_size,
        }
    }

    pub fn with_root(index_size: usize, root: ActVertexId, vertices: Vec<ActVertex>) -> Self {
        Self {
            vertices,
            root,
            index_size,
        }
    }

    pub fn index_size(&self) -> usize {
        self.index_size
    }

    pub fn root(&self) -> ActVertexId {
        self.root
    }

    pub fn set_root(&mut self, root: ActVertexId) {
        self.root = root;
    }

    pub fn add_terminal(&mut self, value: i32) -> ActVertexId {
        self.add_vertex(ActVertex::terminal(self.index_size, value))
    }

    pub fn add_decision(
        &mut self,
        index: usize,
        low: ActVertexId,
        high: ActVertexId,
    ) -> ActVertexId {
        self.add_vertex(ActVertex::decision(index, low, high))
    }

    pub fn add_vertex(&mut self, vertex: ActVertex) -> ActVertexId {
        let id = ActVertexId(self.vertices.len());
        self.vertices.push(vertex);
        id
    }

    pub fn vertex(&self, id: ActVertexId) -> ActReduceResult<&ActVertex> {
        self.vertices
            .get(id.0)
            .ok_or(ActReduceError::MissingVertex(id))
    }

    pub fn vertices(&self) -> &[ActVertex] {
        &self.vertices
    }

    pub fn reachable_vertices(&self) -> ActReduceResult<Vec<ActVertexId>> {
        if self.vertices.is_empty() {
            return Err(ActReduceError::EmptyGraph);
        }

        let mut order = Vec::new();
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        self.collect_reachable(self.root, &mut visiting, &mut visited, &mut order)?;
        Ok(order)
    }

    fn collect_reachable(
        &self,
        id: ActVertexId,
        visiting: &mut HashSet<ActVertexId>,
        visited: &mut HashSet<ActVertexId>,
        order: &mut Vec<ActVertexId>,
    ) -> ActReduceResult<()> {
        self.vertex(id)?;
        if visited.contains(&id) {
            return Ok(());
        }
        if !visiting.insert(id) {
            return Err(ActReduceError::Cycle { vertex: id });
        }

        let vertex = self.vertex(id)?;
        match vertex.kind()? {
            ActVertexKind::Terminal { .. } => {}
            ActVertexKind::Decision { low, high } => {
                self.collect_reachable(low, visiting, visited, order)?;
                self.collect_reachable(high, visiting, visited, order)?;
            }
        }

        visiting.remove(&id);
        visited.insert(id);
        order.push(id);
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActVertex {
    pub index: usize,
    pub value: Option<i32>,
    pub low: Option<ActVertexId>,
    pub high: Option<ActVertexId>,
    pub name: Option<String>,
}

impl ActVertex {
    pub fn terminal(index: usize, value: i32) -> Self {
        Self {
            index,
            value: Some(value),
            low: None,
            high: None,
            name: None,
        }
    }

    pub fn decision(index: usize, low: ActVertexId, high: ActVertexId) -> Self {
        Self {
            index,
            value: None,
            low: Some(low),
            high: Some(high),
            name: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn kind(&self) -> ActReduceResult<ActVertexKind> {
        match (self.value, self.low, self.high) {
            (Some(value), None, None) => Ok(ActVertexKind::Terminal { value }),
            (None, Some(low), Some(high)) => Ok(ActVertexKind::Decision { low, high }),
            _ => Err(ActReduceError::MalformedVertex),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActVertexKind {
    Terminal { value: i32 },
    Decision { low: ActVertexId, high: ActVertexId },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActReduceReport {
    pub removed_equal_child_vertices: usize,
    pub merged_equivalent_vertices: usize,
    pub unreachable_vertices: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReducedAct {
    pub graph: ActGraph,
    pub report: ActReduceReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActReduceError {
    EmptyGraph,
    MissingVertex(ActVertexId),
    MalformedVertex,
    InvalidIndex {
        vertex: ActVertexId,
        index: usize,
        index_size: usize,
    },
    Cycle {
        vertex: ActVertexId,
    },
    MissingReducedChild {
        vertex: ActVertexId,
        child: ActVertexId,
    },
    MissingReducedRoot(ActVertexId),
    MissingNativePorts {
        operation: &'static str,
    },
}

impl fmt::Display for ActReduceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGraph => f.write_str("cannot reduce an empty ACT graph"),
            Self::MissingVertex(vertex) => write!(f, "missing ACT vertex {}", vertex.0),
            Self::MalformedVertex => f.write_str("ACT vertex must be a terminal or a decision"),
            Self::InvalidIndex {
                vertex,
                index,
                index_size,
            } => write!(
                f,
                "ACT vertex {} has index {index}, outside 0..={index_size}",
                vertex.0
            ),
            Self::Cycle { vertex } => write!(
                f,
                "ACT graph contains a cycle involving vertex {}",
                vertex.0
            ),
            Self::MissingReducedChild { vertex, child } => write!(
                f,
                "ACT vertex {} references child {}, which was not reduced",
                vertex.0, child.0
            ),
            Self::MissingReducedRoot(root) => {
                write!(f, "ACT root {} was not present after reduction", root.0)
            }
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation} requires native Rust SIS graph prerequisites"
            ),
        }
    }
}

impl Error for ActReduceError {}

pub type ActReduceResult<T> = Result<T, ActReduceError>;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ReductionKey {
    low: i64,
    high: i64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct QueueEntry {
    key: ReductionKey,
    vertex: ActVertexId,
}

pub fn reduce_act_graph(graph: ActGraph) -> ActReduceResult<ReducedAct> {
    let reachable = graph.reachable_vertices()?;
    let reachable_set: HashSet<_> = reachable.iter().copied().collect();
    let unreachable_vertices = graph.vertices.len().saturating_sub(reachable_set.len());

    let mut by_index = vec![Vec::new(); graph.index_size + 1];
    for id in reachable {
        let vertex = graph.vertex(id)?;
        if vertex.index > graph.index_size {
            return Err(ActReduceError::InvalidIndex {
                vertex: id,
                index: vertex.index,
                index_size: graph.index_size,
            });
        }
        by_index[vertex.index].push(id);
    }

    let mut reduced_ids: Vec<Option<ActVertexId>> = vec![None; graph.vertices.len()];
    let mut reduced_vertices = Vec::new();
    let mut report = ActReduceReport {
        removed_equal_child_vertices: 0,
        merged_equivalent_vertices: 0,
        unreachable_vertices,
    };

    for index in (0..=graph.index_size).rev() {
        let mut queue = Vec::new();
        for id in &by_index[index] {
            let vertex = graph.vertex(*id)?;
            match vertex.kind()? {
                ActVertexKind::Terminal { value } => {
                    queue.push(QueueEntry {
                        key: ReductionKey {
                            low: i64::from(value),
                            high: i64::from(value),
                        },
                        vertex: *id,
                    });
                }
                ActVertexKind::Decision { low, high } => {
                    let low_id = reduced_ids
                        .get(low.0)
                        .and_then(|candidate| *candidate)
                        .ok_or(ActReduceError::MissingReducedChild {
                            vertex: *id,
                            child: low,
                        })?;
                    let high_id = reduced_ids
                        .get(high.0)
                        .and_then(|candidate| *candidate)
                        .ok_or(ActReduceError::MissingReducedChild {
                            vertex: *id,
                            child: high,
                        })?;

                    if low_id == high_id {
                        reduced_ids[id.0] = Some(low_id);
                        report.removed_equal_child_vertices += 1;
                    } else {
                        queue.push(QueueEntry {
                            key: ReductionKey {
                                low: low_id.0 as i64,
                                high: high_id.0 as i64,
                            },
                            vertex: *id,
                        });
                    }
                }
            }
        }

        queue.sort_unstable();
        let mut previous: Option<(ReductionKey, ActVertexId)> = None;
        for entry in queue {
            if let Some((previous_key, representative)) = previous {
                if previous_key == entry.key {
                    reduced_ids[entry.vertex.0] = Some(representative);
                    report.merged_equivalent_vertices += 1;
                    continue;
                }
            }

            let old_vertex = graph.vertex(entry.vertex)?;
            let mut new_vertex = old_vertex.clone();
            if let ActVertexKind::Decision { low, high } = old_vertex.kind()? {
                new_vertex.low = Some(reduced_ids[low.0].ok_or(
                    ActReduceError::MissingReducedChild {
                        vertex: entry.vertex,
                        child: low,
                    },
                )?);
                new_vertex.high = Some(reduced_ids[high.0].ok_or(
                    ActReduceError::MissingReducedChild {
                        vertex: entry.vertex,
                        child: high,
                    },
                )?);
            }

            let new_id = ActVertexId(reduced_vertices.len());
            reduced_vertices.push(new_vertex);
            reduced_ids[entry.vertex.0] = Some(new_id);
            previous = Some((entry.key, new_id));
        }
    }

    let root = reduced_ids
        .get(graph.root.0)
        .and_then(|candidate| *candidate)
        .ok_or(ActReduceError::MissingReducedRoot(graph.root))?;

    Ok(ReducedAct {
        graph: ActGraph::with_root(graph.index_size, root, reduced_vertices),
        report,
    })
}

pub fn reduce_sis_act_graph_blocked<Graph>(_graph: Graph) -> ActReduceResult<ReducedAct> {
    Err(ActReduceError::MissingNativePorts {
        operation: "SIS ACT pointer graph reduction",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduce_merges_equivalent_terminals_and_decisions() {
        let mut graph = ActGraph::new(2);
        let zero_a = graph.add_terminal(0);
        let one_a = graph.add_terminal(1);
        let zero_b = graph.add_terminal(0);
        let one_b = graph.add_terminal(1);
        let left = graph.add_decision(1, zero_a, one_a);
        let right = graph.add_decision(1, zero_b, one_b);
        let root = graph.add_decision(0, left, right);
        graph.set_root(root);

        let reduced = reduce_act_graph(graph).unwrap();

        assert_eq!(reduced.graph.vertices().len(), 3);
        assert_eq!(reduced.report.merged_equivalent_vertices, 3);
        assert_eq!(reduced.report.removed_equal_child_vertices, 1);

        let root_vertex = reduced.graph.vertex(reduced.graph.root()).unwrap();
        assert!(matches!(
            root_vertex.kind().unwrap(),
            ActVertexKind::Decision { .. }
        ));
    }

    #[test]
    fn reduce_bypasses_decisions_with_equal_children() {
        let mut graph = ActGraph::new(1);
        let one = graph.add_terminal(1);
        let redundant = graph.add_decision(0, one, one);
        graph.set_root(redundant);

        let reduced = reduce_act_graph(graph).unwrap();

        assert_eq!(reduced.graph.vertices().len(), 1);
        assert_eq!(reduced.graph.root(), ActVertexId(0));
        assert_eq!(
            reduced.graph.vertex(reduced.graph.root()).unwrap().value,
            Some(1)
        );
        assert_eq!(reduced.report.removed_equal_child_vertices, 1);
    }

    #[test]
    fn reduce_ignores_unreachable_vertices() {
        let mut graph = ActGraph::new(1);
        let zero = graph.add_terminal(0);
        let one = graph.add_terminal(1);
        let root = graph.add_decision(0, zero, one);
        graph.add_decision(0, one, zero);
        graph.set_root(root);

        let reduced = reduce_act_graph(graph).unwrap();

        assert_eq!(reduced.graph.vertices().len(), 3);
        assert_eq!(reduced.report.unreachable_vertices, 1);
    }

    #[test]
    fn reduce_reports_missing_sis_prerequisites_generically() {
        let error = reduce_sis_act_graph_blocked(()).unwrap_err();

        assert!(matches!(
            error,
            ActReduceError::MissingNativePorts {
                operation: "SIS ACT pointer graph reduction"
            }
        ));
    }
}
