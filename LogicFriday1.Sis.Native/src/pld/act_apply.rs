//! Native ACT apply operations over owned decision graphs.
//!
//! The apply routine combines two ACT roots with a Boolean binary operation,
//! memoizes intermediate vertex pairs, and reduces the resulting graph. This
//! module keeps that behavior in Rust-owned data and leaves direct SIS pointer
//! integration to higher-level native graph ports.

use std::collections::{BTreeMap, HashMap, HashSet};
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

    pub fn set_root(&mut self, root: ActVertexId) -> ActApplyResult<()> {
        self.vertex(root)?;
        self.root = root;
        Ok(())
    }

    pub fn add_terminal(&mut self, value: bool) -> ActVertexId {
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

    pub fn vertex(&self, id: ActVertexId) -> ActApplyResult<&ActVertex> {
        self.vertices
            .get(id.0)
            .ok_or(ActApplyError::MissingVertex(id))
    }

    pub fn vertices(&self) -> &[ActVertex] {
        &self.vertices
    }

    fn reachable_vertices(&self) -> ActApplyResult<Vec<ActVertexId>> {
        if self.vertices.is_empty() {
            return Err(ActApplyError::EmptyGraph);
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
    ) -> ActApplyResult<()> {
        self.vertex(id)?;
        if visited.contains(&id) {
            return Ok(());
        }
        if !visiting.insert(id) {
            return Err(ActApplyError::Cycle { vertex: id });
        }

        match self.vertex(id)?.kind()? {
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
    pub value: Option<bool>,
    pub low: Option<ActVertexId>,
    pub high: Option<ActVertexId>,
}

impl ActVertex {
    pub fn terminal(index: usize, value: bool) -> Self {
        Self {
            index,
            value: Some(value),
            low: None,
            high: None,
        }
    }

    pub fn decision(index: usize, low: ActVertexId, high: ActVertexId) -> Self {
        Self {
            index,
            value: None,
            low: Some(low),
            high: Some(high),
        }
    }

    pub fn kind(&self) -> ActApplyResult<ActVertexKind> {
        match (self.value, self.low, self.high) {
            (Some(value), None, None) => Ok(ActVertexKind::Terminal { value }),
            (None, Some(low), Some(high)) => Ok(ActVertexKind::Decision { low, high }),
            _ => Err(ActApplyError::MalformedVertex),
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.value.is_some()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActVertexKind {
    Terminal { value: bool },
    Decision { low: ActVertexId, high: ActVertexId },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputPhase {
    Positive,
    Negative,
}

impl InputPhase {
    fn apply(self, value: bool) -> bool {
        match self {
            Self::Positive => value,
            Self::Negative => !value,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActBinaryOperation {
    And { left: InputPhase, right: InputPhase },
    Or { left: InputPhase, right: InputPhase },
    Xor,
    Xnor,
}

impl ActBinaryOperation {
    pub const AND: Self = Self::And {
        left: InputPhase::Positive,
        right: InputPhase::Positive,
    };
    pub const AND_LEFT_ONLY: Self = Self::And {
        left: InputPhase::Positive,
        right: InputPhase::Negative,
    };
    pub const AND_RIGHT_ONLY: Self = Self::And {
        left: InputPhase::Negative,
        right: InputPhase::Positive,
    };
    pub const NOR: Self = Self::And {
        left: InputPhase::Negative,
        right: InputPhase::Negative,
    };
    pub const OR: Self = Self::Or {
        left: InputPhase::Positive,
        right: InputPhase::Positive,
    };
    pub const OR_LEFT_OR_NOT_RIGHT: Self = Self::Or {
        left: InputPhase::Positive,
        right: InputPhase::Negative,
    };
    pub const OR_NOT_LEFT_OR_RIGHT: Self = Self::Or {
        left: InputPhase::Negative,
        right: InputPhase::Positive,
    };
    pub const NAND: Self = Self::Or {
        left: InputPhase::Negative,
        right: InputPhase::Negative,
    };

    fn evaluate(self, left: bool, right: bool) -> bool {
        match self {
            Self::And { left: l, right: r } => l.apply(left) && r.apply(right),
            Self::Or { left: l, right: r } => l.apply(left) || r.apply(right),
            Self::Xor => left ^ right,
            Self::Xnor => left == right,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyReport {
    pub memoized_pairs: usize,
    pub removed_equal_child_vertices: usize,
    pub merged_equivalent_vertices: usize,
    pub unreachable_vertices: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppliedAct {
    pub graph: ActGraph,
    pub report: ApplyReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActApplyError {
    EmptyGraph,
    MissingVertex(ActVertexId),
    MalformedVertex,
    IndexSizeMismatch {
        left: usize,
        right: usize,
    },
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

impl fmt::Display for ActApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGraph => f.write_str("cannot apply an empty ACT graph"),
            Self::MissingVertex(vertex) => write!(f, "missing ACT vertex {}", vertex.0),
            Self::MalformedVertex => f.write_str("ACT vertex must be a terminal or a decision"),
            Self::IndexSizeMismatch { left, right } => {
                write!(
                    f,
                    "ACT graph index sizes differ: left {left}, right {right}"
                )
            }
            Self::InvalidIndex {
                vertex,
                index,
                index_size,
            } => write!(
                f,
                "ACT vertex {} has index {index}, outside 0..={index_size}",
                vertex.0
            ),
            Self::Cycle { vertex } => {
                write!(
                    f,
                    "ACT graph contains a cycle involving vertex {}",
                    vertex.0
                )
            }
            Self::MissingReducedChild { vertex, child } => write!(
                f,
                "ACT vertex {} references child {}, which was not reduced",
                vertex.0, child.0
            ),
            Self::MissingReducedRoot(root) => {
                write!(f, "ACT root {} was not present after reduction", root.0)
            }
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native SIS prerequisite ports")
            }
        }
    }
}

impl Error for ActApplyError {}

pub type ActApplyResult<T> = Result<T, ActApplyError>;

pub fn apply_act_graphs(
    left: &ActGraph,
    right: &ActGraph,
    operation: ActBinaryOperation,
) -> ActApplyResult<AppliedAct> {
    if left.index_size() != right.index_size() {
        return Err(ActApplyError::IndexSizeMismatch {
            left: left.index_size(),
            right: right.index_size(),
        });
    }

    left.vertex(left.root())?;
    right.vertex(right.root())?;

    let mut builder = ApplyBuilder {
        left,
        right,
        operation,
        result: ActGraph::new(left.index_size()),
        memo: HashMap::new(),
    };
    let root = builder.apply_step(left.root(), right.root())?;
    builder.result.set_root(root)?;
    let memoized_pairs = builder.memo.len();

    let reduced = reduce_act_graph(builder.result)?;
    Ok(AppliedAct {
        graph: reduced.graph,
        report: ApplyReport {
            memoized_pairs,
            removed_equal_child_vertices: reduced.report.removed_equal_child_vertices,
            merged_equivalent_vertices: reduced.report.merged_equivalent_vertices,
            unreachable_vertices: reduced.report.unreachable_vertices,
        },
    })
}

pub fn apply_sis_act_graphs_blocked<Graph>(
    _left: &Graph,
    _right: &Graph,
    _operation: ActBinaryOperation,
) -> ActApplyResult<AppliedAct> {
    Err(ActApplyError::MissingNativePorts {
        operation: "SIS ACT graph apply",
    })
}

pub fn evaluate_graph(graph: &ActGraph, inputs: &[bool]) -> ActApplyResult<bool> {
    if inputs.len() != graph.index_size() {
        return Err(ActApplyError::IndexSizeMismatch {
            left: graph.index_size(),
            right: inputs.len(),
        });
    }

    evaluate_vertex(graph, graph.root(), inputs)
}

struct ApplyBuilder<'a> {
    left: &'a ActGraph,
    right: &'a ActGraph,
    operation: ActBinaryOperation,
    result: ActGraph,
    memo: HashMap<(ActVertexId, ActVertexId), ActVertexId>,
}

impl ApplyBuilder<'_> {
    fn apply_step(
        &mut self,
        left_id: ActVertexId,
        right_id: ActVertexId,
    ) -> ActApplyResult<ActVertexId> {
        if let Some(id) = self.memo.get(&(left_id, right_id)) {
            return Ok(*id);
        }

        let left = self.left.vertex(left_id)?;
        let right = self.right.vertex(right_id)?;
        let id = if let (
            ActVertexKind::Terminal { value: left_value },
            ActVertexKind::Terminal { value: right_value },
        ) = (left.kind()?, right.kind()?)
        {
            self.result
                .add_terminal(self.operation.evaluate(left_value, right_value))
        } else {
            let index = next_index(left, right);
            validate_index(left_id, index, self.left.index_size())?;
            validate_index(right_id, index, self.right.index_size())?;

            let left_children = children_for_index(left, left_id, index)?;
            let right_children = children_for_index(right, right_id, index)?;
            let low = self.apply_step(left_children.low, right_children.low)?;
            let high = self.apply_step(left_children.high, right_children.high)?;
            self.result.add_decision(index, low, high)
        };

        self.memo.insert((left_id, right_id), id);
        Ok(id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BranchChildren {
    low: ActVertexId,
    high: ActVertexId,
}

fn children_for_index(
    vertex: &ActVertex,
    id: ActVertexId,
    index: usize,
) -> ActApplyResult<BranchChildren> {
    if vertex.index == index && !vertex.is_terminal() {
        match vertex.kind()? {
            ActVertexKind::Decision { low, high } => Ok(BranchChildren { low, high }),
            ActVertexKind::Terminal { .. } => Ok(BranchChildren { low: id, high: id }),
        }
    } else {
        Ok(BranchChildren { low: id, high: id })
    }
}

fn next_index(left: &ActVertex, right: &ActVertex) -> usize {
    match (left.is_terminal(), right.is_terminal()) {
        (true, true) => left.index.min(right.index),
        (true, false) => right.index,
        (false, true) => left.index,
        (false, false) => left.index.min(right.index),
    }
}

fn validate_index(id: ActVertexId, index: usize, index_size: usize) -> ActApplyResult<()> {
    if index > index_size {
        Err(ActApplyError::InvalidIndex {
            vertex: id,
            index,
            index_size,
        })
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReduceReport {
    removed_equal_child_vertices: usize,
    merged_equivalent_vertices: usize,
    unreachable_vertices: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReducedAct {
    graph: ActGraph,
    report: ReduceReport,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ReductionKey {
    Terminal(bool),
    Decision {
        index: usize,
        low: ActVertexId,
        high: ActVertexId,
    },
}

fn reduce_act_graph(graph: ActGraph) -> ActApplyResult<ReducedAct> {
    let reachable = graph.reachable_vertices()?;
    let reachable_set: HashSet<_> = reachable.iter().copied().collect();
    let mut reduced_ids: Vec<Option<ActVertexId>> = vec![None; graph.vertices.len()];
    let mut reduced = ActGraph::new(graph.index_size());
    let mut canonical = BTreeMap::new();
    let mut report = ReduceReport {
        removed_equal_child_vertices: 0,
        merged_equivalent_vertices: 0,
        unreachable_vertices: graph.vertices.len().saturating_sub(reachable_set.len()),
    };

    for id in reachable {
        let vertex = graph.vertex(id)?;
        let key = match vertex.kind()? {
            ActVertexKind::Terminal { value } => ReductionKey::Terminal(value),
            ActVertexKind::Decision { low, high } => {
                let low_id = reduced_ids
                    .get(low.0)
                    .and_then(|candidate| *candidate)
                    .ok_or(ActApplyError::MissingReducedChild {
                        vertex: id,
                        child: low,
                    })?;
                let high_id = reduced_ids
                    .get(high.0)
                    .and_then(|candidate| *candidate)
                    .ok_or(ActApplyError::MissingReducedChild {
                        vertex: id,
                        child: high,
                    })?;

                if low_id == high_id {
                    reduced_ids[id.0] = Some(low_id);
                    report.removed_equal_child_vertices += 1;
                    continue;
                }

                ReductionKey::Decision {
                    index: vertex.index,
                    low: low_id,
                    high: high_id,
                }
            }
        };

        if let Some(existing) = canonical.get(&key).copied() {
            reduced_ids[id.0] = Some(existing);
            report.merged_equivalent_vertices += 1;
            continue;
        }

        let new_id = match key {
            ReductionKey::Terminal(value) => reduced.add_terminal(value),
            ReductionKey::Decision { index, low, high } => reduced.add_decision(index, low, high),
        };
        canonical.insert(key, new_id);
        reduced_ids[id.0] = Some(new_id);
    }

    let root = reduced_ids
        .get(graph.root().0)
        .and_then(|candidate| *candidate)
        .ok_or(ActApplyError::MissingReducedRoot(graph.root()))?;
    reduced.set_root(root)?;
    Ok(ReducedAct {
        graph: reduced,
        report,
    })
}

fn evaluate_vertex(graph: &ActGraph, id: ActVertexId, inputs: &[bool]) -> ActApplyResult<bool> {
    let vertex = graph.vertex(id)?;
    match vertex.kind()? {
        ActVertexKind::Terminal { value } => Ok(value),
        ActVertexKind::Decision { low, high } => {
            let input = inputs
                .get(vertex.index)
                .ok_or(ActApplyError::InvalidIndex {
                    vertex: id,
                    index: vertex.index,
                    index_size: graph.index_size(),
                })?;
            if *input {
                evaluate_vertex(graph, high, inputs)
            } else {
                evaluate_vertex(graph, low, inputs)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn literal_graph(index: usize, index_size: usize) -> ActGraph {
        let mut graph = ActGraph::new(index_size);
        let zero = graph.add_terminal(false);
        let one = graph.add_terminal(true);
        let root = graph.add_decision(index, zero, one);
        graph.set_root(root).unwrap();
        graph
    }

    fn constant_graph(value: bool, index_size: usize) -> ActGraph {
        let mut graph = ActGraph::new(index_size);
        let root = graph.add_terminal(value);
        graph.set_root(root).unwrap();
        graph
    }

    fn assert_matches_operation(
        left: &ActGraph,
        right: &ActGraph,
        operation: ActBinaryOperation,
        applied: &AppliedAct,
    ) {
        let index_size = left.index_size();
        for mask in 0usize..(1usize << index_size) {
            let inputs: Vec<_> = (0..index_size)
                .map(|index| (mask & (1usize << index)) != 0)
                .collect();
            let expected = operation.evaluate(
                evaluate_graph(left, &inputs).unwrap(),
                evaluate_graph(right, &inputs).unwrap(),
            );
            assert_eq!(evaluate_graph(&applied.graph, &inputs).unwrap(), expected);
        }
    }

    #[test]
    fn apply_and_builds_reduced_decision_graph() {
        let a = literal_graph(0, 2);
        let b = literal_graph(1, 2);

        let applied = apply_act_graphs(&a, &b, ActBinaryOperation::AND).unwrap();

        assert_matches_operation(&a, &b, ActBinaryOperation::AND, &applied);
        assert!(applied.report.memoized_pairs > 0);
        assert_eq!(applied.graph.vertices().len(), 4);
    }

    #[test]
    fn apply_supports_legacy_phase_variants() {
        let a = literal_graph(0, 2);
        let b = literal_graph(1, 2);

        let applied = apply_act_graphs(&a, &b, ActBinaryOperation::NAND).unwrap();

        assert_matches_operation(&a, &b, ActBinaryOperation::NAND, &applied);
        assert!(!evaluate_graph(&applied.graph, &[true, true]).unwrap());
        assert!(evaluate_graph(&applied.graph, &[false, true]).unwrap());
    }

    #[test]
    fn apply_xor_and_xnor_match_truth_tables() {
        let a = literal_graph(0, 2);
        let b = literal_graph(1, 2);
        let xor = apply_act_graphs(&a, &b, ActBinaryOperation::Xor).unwrap();
        let xnor = apply_act_graphs(&a, &b, ActBinaryOperation::Xnor).unwrap();

        assert_matches_operation(&a, &b, ActBinaryOperation::Xor, &xor);
        assert_matches_operation(&a, &b, ActBinaryOperation::Xnor, &xnor);
        assert!(evaluate_graph(&xor.graph, &[true, false]).unwrap());
        assert!(!evaluate_graph(&xnor.graph, &[true, false]).unwrap());
    }

    #[test]
    fn apply_reduces_constant_results() {
        let a = literal_graph(0, 1);
        let one = constant_graph(true, 1);

        let applied = apply_act_graphs(&a, &one, ActBinaryOperation::OR).unwrap();

        assert_eq!(applied.graph.vertices(), &[ActVertex::terminal(1, true)]);
        assert_eq!(applied.report.removed_equal_child_vertices, 1);
    }

    #[test]
    fn apply_reuses_memoized_pairs_for_shared_children() {
        let mut graph = ActGraph::new(1);
        let zero = graph.add_terminal(false);
        let one = graph.add_terminal(true);
        let root = graph.add_decision(0, zero, one);
        graph.set_root(root).unwrap();

        let applied = apply_act_graphs(
            &graph,
            &graph,
            ActBinaryOperation::And {
                left: InputPhase::Positive,
                right: InputPhase::Negative,
            },
        )
        .unwrap();

        assert_eq!(applied.graph.vertices(), &[ActVertex::terminal(1, false)]);
        assert_eq!(applied.report.memoized_pairs, 3);
    }

    #[test]
    fn malformed_graphs_are_reported() {
        let malformed = ActGraph::with_root(
            1,
            ActVertexId(0),
            vec![ActVertex {
                index: 0,
                value: None,
                low: None,
                high: None,
            }],
        );
        let one = constant_graph(true, 1);

        assert_eq!(
            apply_act_graphs(&malformed, &one, ActBinaryOperation::AND),
            Err(ActApplyError::MalformedVertex)
        );
    }

    #[test]
    fn blocked_sis_integration_reports_generic_diagnostic() {
        let error = apply_sis_act_graphs_blocked(&(), &(), ActBinaryOperation::AND).unwrap_err();

        assert_eq!(
            error,
            ActApplyError::MissingNativePorts {
                operation: "SIS ACT graph apply",
            }
        );
        assert!(!error.to_string().contains(concat!("Logic", "Friday1", "-")));
    }

    #[test]
    fn no_legacy_c_abi_or_dependency_metadata_tokens_are_present() {
        let text = include_str!("act_apply.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
        assert!(!text.contains(concat!("Logic", "Friday1", "-")));
    }
}
