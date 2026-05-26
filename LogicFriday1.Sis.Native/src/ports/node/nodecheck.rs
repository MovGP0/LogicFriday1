//! Native consistency checks for SIS-style Boolean nodes.
//!
//! The legacy checker validated internal node covers and returned the first
//! detected inconsistency. This port exposes the same checks as structured Rust
//! diagnostics over typed cubes, so invalid literal encodings are retired by
//! construction.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckNodeKind {
    Internal,
    PrimaryInput,
    PrimaryOutput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckCube {
    inputs: Vec<Option<bool>>,
}

impl CheckCube {
    pub fn new(inputs: Vec<Option<bool>>) -> Self {
        Self { inputs }
    }

    pub fn tautology(input_count: usize) -> Self {
        Self {
            inputs: vec![None; input_count],
        }
    }

    pub fn inputs(&self) -> &[Option<bool>] {
        &self.inputs
    }

    pub fn input_count(&self) -> usize {
        self.inputs.len()
    }

    pub fn covers(&self, other: &Self) -> bool {
        self.inputs
            .iter()
            .zip(&other.inputs)
            .all(|(left, right)| left.is_none() || left == right)
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.inputs
            .iter()
            .zip(&other.inputs)
            .all(|(left, right)| match (left, right) {
                (Some(left), Some(right)) => left == right,
                _ => true,
            })
    }

    fn evaluate(&self, assignment: &[bool]) -> bool {
        self.inputs
            .iter()
            .zip(assignment)
            .all(|(input, value)| input.map_or(true, |required| required == *value))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckCover {
    input_count: usize,
    cubes: Vec<CheckCube>,
}

impl CheckCover {
    pub fn new(input_count: usize, cubes: Vec<CheckCube>) -> NodeCheckResult<Self> {
        if let Some(cube) = cubes.iter().find(|cube| cube.input_count() != input_count) {
            return Err(NodeCheckError::CoverInputCountMismatch {
                expected: input_count,
                actual: cube.input_count(),
            });
        }

        Ok(Self { input_count, cubes })
    }

    pub fn zero(input_count: usize) -> Self {
        Self {
            input_count,
            cubes: Vec::new(),
        }
    }

    pub fn one(input_count: usize) -> Self {
        Self {
            input_count,
            cubes: vec![CheckCube::tautology(input_count)],
        }
    }

    pub fn input_count(&self) -> usize {
        self.input_count
    }

    pub fn cubes(&self) -> &[CheckCube] {
        &self.cubes
    }

    fn evaluate(&self, assignment: &[bool]) -> bool {
        self.cubes.iter().any(|cube| cube.evaluate(assignment))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckNode {
    pub name: Option<String>,
    pub kind: CheckNodeKind,
    pub fanins: Vec<String>,
    function: Option<CheckCover>,
    offset: Option<CheckCover>,
}

impl CheckNode {
    pub fn internal(
        name: Option<String>,
        fanins: Vec<String>,
        function: Option<CheckCover>,
    ) -> Self {
        Self {
            name,
            kind: CheckNodeKind::Internal,
            fanins,
            function,
            offset: None,
        }
    }

    pub fn with_offset(mut self, offset: CheckCover) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn primary_input(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            kind: CheckNodeKind::PrimaryInput,
            fanins: Vec::new(),
            function: None,
            offset: None,
        }
    }

    pub fn primary_output(name: impl Into<String>, fanin: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            kind: CheckNodeKind::PrimaryOutput,
            fanins: vec![fanin.into()],
            function: None,
            offset: None,
        }
    }

    pub fn function(&self) -> Option<&CheckCover> {
        self.function.as_ref()
    }

    pub fn offset(&self) -> Option<&CheckCover> {
        self.offset.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeCheckError {
    MissingFunction {
        node_name: Option<String>,
    },
    FunctionIsNotMinimumBase {
        node_name: Option<String>,
        fanin_index: usize,
    },
    FunctionIsNotSccMinimal {
        node_name: Option<String>,
        covering_cube: usize,
        covered_cube: usize,
    },
    CoverInputCountMismatch {
        expected: usize,
        actual: usize,
    },
    OnsetAndOffsetIntersect {
        node_name: Option<String>,
        onset_cube: usize,
        offset_cube: usize,
    },
    OnsetAndOffsetAreIncomplete {
        node_name: Option<String>,
    },
}

impl fmt::Display for NodeCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFunction { node_name } => {
                write_node_prefix(f, node_name)?;
                write!(f, "internal node does not have a logic function")
            }
            Self::FunctionIsNotMinimumBase {
                node_name,
                fanin_index,
            } => {
                write_node_prefix(f, node_name)?;
                write!(f, "node is not minimum base at fanin {fanin_index}")
            }
            Self::FunctionIsNotSccMinimal {
                node_name,
                covering_cube,
                covered_cube,
            } => {
                write_node_prefix(f, node_name)?;
                write!(
                    f,
                    "node is not SCC-minimal: cube {covering_cube} covers cube {covered_cube}"
                )
            }
            Self::CoverInputCountMismatch { expected, actual } => {
                write!(
                    f,
                    "cover has {actual} inputs, expected {expected} inputs for this node"
                )
            }
            Self::OnsetAndOffsetIntersect {
                node_name,
                onset_cube,
                offset_cube,
            } => {
                write_node_prefix(f, node_name)?;
                write!(
                    f,
                    "node onset cube {onset_cube} intersects offset cube {offset_cube}"
                )
            }
            Self::OnsetAndOffsetAreIncomplete { node_name } => {
                write_node_prefix(f, node_name)?;
                write!(f, "missing minterms from onset union offset")
            }
        }
    }
}

impl Error for NodeCheckError {}

pub type NodeCheckResult<T> = Result<T, NodeCheckError>;

pub fn node_check(node: &CheckNode) -> NodeCheckResult<()> {
    if node.kind != CheckNodeKind::Internal {
        return Ok(());
    }

    let Some(function) = node.function() else {
        return Err(NodeCheckError::MissingFunction {
            node_name: node.name.clone(),
        });
    };

    check_cover_shape(function, node.fanins.len())?;
    check_minimum_base(node, function)?;
    check_scc_minimal(node, function)?;

    if let Some(offset) = node.offset() {
        check_cover_shape(offset, node.fanins.len())?;
        check_onset_offset_partition(node, function, offset)?;
    }

    Ok(())
}

fn check_cover_shape(cover: &CheckCover, expected: usize) -> NodeCheckResult<()> {
    if cover.input_count() != expected {
        return Err(NodeCheckError::CoverInputCountMismatch {
            expected,
            actual: cover.input_count(),
        });
    }

    Ok(())
}

fn check_minimum_base(node: &CheckNode, function: &CheckCover) -> NodeCheckResult<()> {
    for index in 0..node.fanins.len() {
        if function
            .cubes()
            .iter()
            .all(|cube| cube.inputs()[index].is_none())
        {
            return Err(NodeCheckError::FunctionIsNotMinimumBase {
                node_name: node.name.clone(),
                fanin_index: index,
            });
        }
    }

    Ok(())
}

fn check_scc_minimal(node: &CheckNode, function: &CheckCover) -> NodeCheckResult<()> {
    for (left_index, left) in function.cubes().iter().enumerate() {
        for (right_index, right) in function.cubes().iter().enumerate() {
            if left_index != right_index && left.covers(right) {
                return Err(NodeCheckError::FunctionIsNotSccMinimal {
                    node_name: node.name.clone(),
                    covering_cube: left_index,
                    covered_cube: right_index,
                });
            }
        }
    }

    Ok(())
}

fn check_onset_offset_partition(
    node: &CheckNode,
    function: &CheckCover,
    offset: &CheckCover,
) -> NodeCheckResult<()> {
    for (onset_index, onset_cube) in function.cubes().iter().enumerate() {
        for (offset_index, offset_cube) in offset.cubes().iter().enumerate() {
            if onset_cube.intersects(offset_cube) {
                return Err(NodeCheckError::OnsetAndOffsetIntersect {
                    node_name: node.name.clone(),
                    onset_cube: onset_index,
                    offset_cube: offset_index,
                });
            }
        }
    }

    let mut complete = true;
    visit_assignments(function.input_count(), &mut Vec::new(), &mut |assignment| {
        if !function.evaluate(assignment) && !offset.evaluate(assignment) {
            complete = false;
        }
    });

    if complete {
        Ok(())
    } else {
        Err(NodeCheckError::OnsetAndOffsetAreIncomplete {
            node_name: node.name.clone(),
        })
    }
}

fn visit_assignments<F>(input_count: usize, partial: &mut Vec<bool>, visit: &mut F)
where
    F: FnMut(&[bool]),
{
    if partial.len() == input_count {
        visit(partial);
        return;
    }

    partial.push(false);
    visit_assignments(input_count, partial, visit);
    partial.pop();
    partial.push(true);
    visit_assignments(input_count, partial, visit);
    partial.pop();
}

fn write_node_prefix(f: &mut fmt::Formatter<'_>, node_name: &Option<String>) -> fmt::Result {
    if let Some(node_name) = node_name {
        write!(f, "{node_name}: ")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cover(input_count: usize, cubes: Vec<Vec<Option<bool>>>) -> CheckCover {
        CheckCover::new(
            input_count,
            cubes.into_iter().map(CheckCube::new).collect::<Vec<_>>(),
        )
        .unwrap()
    }

    fn node(function: Option<CheckCover>, fanins: &[&str]) -> CheckNode {
        CheckNode::internal(
            Some("n".to_string()),
            fanins.iter().map(|fanin| fanin.to_string()).collect(),
            function,
        )
    }

    #[test]
    fn accepts_minimum_base_scc_minimal_internal_node() {
        let node = node(
            Some(cover(
                2,
                vec![vec![Some(true), Some(false)], vec![Some(false), Some(true)]],
            )),
            &["a", "b"],
        );

        assert_eq!(node_check(&node), Ok(()));
    }

    #[test]
    fn ignores_non_internal_nodes_without_logic_function() {
        assert_eq!(node_check(&CheckNode::primary_input("a")), Ok(()));
        assert_eq!(node_check(&CheckNode::primary_output("out", "a")), Ok(()));
    }

    #[test]
    fn rejects_internal_node_without_function() {
        let node = node(None, &["a"]);

        assert_eq!(
            node_check(&node),
            Err(NodeCheckError::MissingFunction {
                node_name: Some("n".to_string())
            })
        );
    }

    #[test]
    fn rejects_unused_fanin() {
        let node = node(Some(cover(2, vec![vec![Some(true), None]])), &["a", "b"]);

        assert_eq!(
            node_check(&node),
            Err(NodeCheckError::FunctionIsNotMinimumBase {
                node_name: Some("n".to_string()),
                fanin_index: 1
            })
        );
    }

    #[test]
    fn rejects_cube_containment() {
        let node = node(
            Some(cover(
                2,
                vec![vec![Some(true), None], vec![Some(true), Some(false)]],
            )),
            &["a", "b"],
        );

        assert_eq!(
            node_check(&node),
            Err(NodeCheckError::FunctionIsNotSccMinimal {
                node_name: Some("n".to_string()),
                covering_cube: 0,
                covered_cube: 1
            })
        );
    }

    #[test]
    fn accepts_complete_disjoint_onset_offset_partition() {
        let node = node(Some(cover(1, vec![vec![Some(true)]])), &["a"])
            .with_offset(cover(1, vec![vec![Some(false)]]));

        assert_eq!(node_check(&node), Ok(()));
    }

    #[test]
    fn rejects_intersecting_onset_and_offset() {
        let node = node(Some(cover(1, vec![vec![Some(true)]])), &["a"])
            .with_offset(cover(1, vec![vec![None]]));

        assert_eq!(
            node_check(&node),
            Err(NodeCheckError::OnsetAndOffsetIntersect {
                node_name: Some("n".to_string()),
                onset_cube: 0,
                offset_cube: 0
            })
        );
    }

    #[test]
    fn rejects_incomplete_onset_offset_partition() {
        let node = node(
            Some(cover(2, vec![vec![Some(true), Some(true)]])),
            &["a", "b"],
        )
        .with_offset(cover(2, vec![vec![Some(false), Some(false)]]));

        assert_eq!(
            node_check(&node),
            Err(NodeCheckError::OnsetAndOffsetAreIncomplete {
                node_name: Some("n".to_string())
            })
        );
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_metadata_tokens_are_present() {
        let source = include_str!("nodecheck.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-")));
    }
}
