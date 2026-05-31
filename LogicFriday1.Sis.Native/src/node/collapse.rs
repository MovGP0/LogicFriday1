//! Native Rust cover collapse for SIS-style nodes.
//!
//! The implementation keeps the original operation in owned Rust data: collapse
//! an internal fanin node into an internal fanout node, preserve the fast
//! constant cases, and rewrite the fanout cover over the collapsed fanin base.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Literal {
    Zero,
    One,
    DontCare,
}

impl Literal {
    fn matches_value(self, value: bool) -> bool {
        match self {
            Self::Zero => !value,
            Self::One => value,
            Self::DontCare => true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    literals: Vec<Literal>,
}

impl Cube {
    pub fn new(literals: Vec<Literal>) -> Self {
        Self { literals }
    }

    pub fn one(input_count: usize) -> Self {
        Self {
            literals: vec![Literal::DontCare; input_count],
        }
    }

    pub fn literals(&self) -> &[Literal] {
        &self.literals
    }

    fn without_literal(&self, index: usize) -> Self {
        let mut literals = self.literals.clone();
        literals.remove(index);
        Self { literals }
    }

    fn is_superset_of(&self, other: &Self) -> bool {
        self.literals
            .iter()
            .zip(&other.literals)
            .all(|(left, right)| *left == Literal::DontCare || left == right)
    }

    fn matches_assignment(&self, assignment: &[bool]) -> bool {
        self.literals
            .iter()
            .zip(assignment)
            .all(|(literal, value)| literal.matches_value(*value))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    PrimaryInput,
    PrimaryOutput,
    Zero,
    One,
    Buffer,
    Inverter,
    And,
    Or,
    Complex,
    Undefined,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub cover: Vec<Cube>,
}

impl CoverNode {
    pub fn new(
        id: NodeId,
        name: impl Into<String>,
        kind: NodeKind,
        fanins: Vec<NodeId>,
        cover: Vec<Cube>,
    ) -> CollapseResult<Self> {
        let node = Self {
            id,
            name: name.into(),
            kind,
            fanins,
            cover,
        };
        node.validate_cover()?;
        Ok(node)
    }

    pub fn primary_input(id: NodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            kind: NodeKind::PrimaryInput,
            fanins: Vec::new(),
            cover: Vec::new(),
        }
    }

    pub fn primary_output(
        id: NodeId,
        name: impl Into<String>,
        fanin: NodeId,
    ) -> CollapseResult<Self> {
        Self::new(id, name, NodeKind::PrimaryOutput, vec![fanin], Vec::new())
    }

    pub fn internal(
        id: NodeId,
        name: impl Into<String>,
        fanins: Vec<NodeId>,
        cover: Vec<Cube>,
    ) -> CollapseResult<Self> {
        Self::new(id, name, NodeKind::Internal, fanins, cover)
    }

    pub fn constant(id: NodeId, value: bool) -> Self {
        Self {
            id,
            name: String::new(),
            kind: NodeKind::Internal,
            fanins: Vec::new(),
            cover: value.then(|| Cube::one(0)).into_iter().collect(),
        }
    }

    pub fn function(&self) -> CollapseResult<NodeFunction> {
        if self.kind == NodeKind::PrimaryInput {
            return Ok(NodeFunction::PrimaryInput);
        }
        if self.kind == NodeKind::PrimaryOutput {
            return Ok(NodeFunction::PrimaryOutput);
        }

        self.validate_cover()?;
        if self.cover.is_empty() {
            if self.fanins.is_empty() {
                return Ok(NodeFunction::Zero);
            }
            return Ok(NodeFunction::Undefined);
        }
        if self.cover.len() == 1 {
            let cube = &self.cover[0];
            if self.fanins.is_empty() {
                return Ok(NodeFunction::One);
            }
            if self.fanins.len() == 1 {
                return match cube.literals[0] {
                    Literal::One => Ok(NodeFunction::Buffer),
                    Literal::Zero => Ok(NodeFunction::Inverter),
                    Literal::DontCare => Ok(NodeFunction::One),
                };
            }
            if cube
                .literals
                .iter()
                .all(|literal| *literal != Literal::DontCare)
            {
                return Ok(NodeFunction::And);
            }
        }

        if self.cover.iter().all(|cube| {
            cube.literals
                .iter()
                .filter(|literal| **literal != Literal::DontCare)
                .count()
                == 1
        }) {
            return Ok(NodeFunction::Or);
        }

        Ok(NodeFunction::Complex)
    }

    pub fn fanin_index(&self, fanin: NodeId) -> Option<usize> {
        self.fanins.iter().position(|candidate| *candidate == fanin)
    }

    fn validate_cover(&self) -> CollapseResult<()> {
        for cube in &self.cover {
            if cube.literals.len() != self.fanins.len() {
                return Err(CollapseError::CoverArityMismatch {
                    node: self.id,
                    fanins: self.fanins.len(),
                    literals: cube.literals.len(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CollapseError {
    CoverArityMismatch {
        node: NodeId,
        fanins: usize,
        literals: usize,
    },
    DuplicateFanin(NodeId),
    UnknownFanin(NodeId),
}

impl fmt::Display for CollapseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoverArityMismatch {
                node,
                fanins,
                literals,
            } => write!(
                f,
                "node {:?} has {fanins} fanins but a cube with {literals} literals",
                node
            ),
            Self::DuplicateFanin(node) => write!(f, "duplicate fanin {:?}", node),
            Self::UnknownFanin(node) => write!(f, "unknown fanin {:?}", node),
        }
    }
}

impl Error for CollapseError {}

pub type CollapseResult<T> = Result<T, CollapseError>;

pub fn collapse_node(fanout: &mut CoverNode, collapsed: &CoverNode) -> CollapseResult<bool> {
    if fanout.kind != NodeKind::Internal || collapsed.kind != NodeKind::Internal {
        return Ok(false);
    }

    let Some(index) = fanout.fanin_index(collapsed.id) else {
        return Ok(false);
    };

    match collapsed.function()? {
        NodeFunction::Zero => {
            handle_constant(fanout, index, Literal::One)?;
            Ok(true)
        }
        NodeFunction::One => {
            handle_constant(fanout, index, Literal::Zero)?;
            Ok(true)
        }
        _ => {
            collapse_general(fanout, collapsed, index)?;
            Ok(true)
        }
    }
}

fn handle_constant(fanout: &mut CoverNode, index: usize, bad_value: Literal) -> CollapseResult<()> {
    let mut cover = Vec::new();
    for cube in &fanout.cover {
        if cube.literals[index] != bad_value {
            cover.push(cube.without_literal(index));
        }
    }

    fanout.fanins.remove(index);
    fanout.cover = minimize_cover(cover);
    fanout.validate_cover()
}

fn collapse_general(
    fanout: &mut CoverNode,
    collapsed: &CoverNode,
    index: usize,
) -> CollapseResult<()> {
    let fanins = collapsed_fanin_base(fanout, collapsed, index)?;
    let assignments = satisfying_assignments(fanout, collapsed, index, &fanins)?;

    fanout.fanins = fanins;
    fanout.cover = minimize_cover(
        assignments
            .into_iter()
            .map(|assignment| {
                Cube::new(
                    assignment
                        .into_iter()
                        .map(|value| if value { Literal::One } else { Literal::Zero })
                        .collect(),
                )
            })
            .collect(),
    );
    fanout.validate_cover()
}

fn collapsed_fanin_base(
    fanout: &CoverNode,
    collapsed: &CoverNode,
    index: usize,
) -> CollapseResult<Vec<NodeId>> {
    let mut fanins = Vec::new();
    for (fanin_index, fanin) in fanout.fanins.iter().copied().enumerate() {
        if fanin_index == index {
            for replacement in &collapsed.fanins {
                push_unique(&mut fanins, *replacement)?;
            }
        } else {
            push_unique(&mut fanins, fanin)?;
        }
    }
    Ok(fanins)
}

fn push_unique(fanins: &mut Vec<NodeId>, fanin: NodeId) -> CollapseResult<()> {
    if fanins.contains(&fanin) {
        return Ok(());
    }
    fanins.push(fanin);
    Ok(())
}

fn satisfying_assignments(
    fanout: &CoverNode,
    collapsed: &CoverNode,
    collapsed_index: usize,
    fanins: &[NodeId],
) -> CollapseResult<Vec<Vec<bool>>> {
    let mut assignments = Vec::new();
    let mut current = vec![false; fanins.len()];
    collect_assignments(0, fanins, &mut current, &mut |assignment| {
        if evaluates_collapsed(fanout, collapsed, collapsed_index, fanins, assignment)? {
            assignments.push(assignment.to_vec());
        }
        Ok(())
    })?;
    Ok(assignments)
}

fn collect_assignments<F>(
    index: usize,
    fanins: &[NodeId],
    current: &mut [bool],
    visit: &mut F,
) -> CollapseResult<()>
where
    F: FnMut(&[bool]) -> CollapseResult<()>,
{
    if index == fanins.len() {
        return visit(current);
    }

    current[index] = false;
    collect_assignments(index + 1, fanins, current, visit)?;
    current[index] = true;
    collect_assignments(index + 1, fanins, current, visit)
}

fn evaluates_collapsed(
    fanout: &CoverNode,
    collapsed: &CoverNode,
    collapsed_index: usize,
    fanins: &[NodeId],
    assignment: &[bool],
) -> CollapseResult<bool> {
    let collapsed_value = evaluate_node(collapsed, fanins, assignment)?;
    let mut fanout_assignment = Vec::with_capacity(fanout.fanins.len());
    for (index, fanin) in fanout.fanins.iter().copied().enumerate() {
        if index == collapsed_index {
            fanout_assignment.push(collapsed_value);
        } else {
            fanout_assignment.push(value_for_fanin(fanin, fanins, assignment)?);
        }
    }
    Ok(evaluate_cover(&fanout.cover, &fanout_assignment))
}

fn evaluate_node(node: &CoverNode, fanins: &[NodeId], assignment: &[bool]) -> CollapseResult<bool> {
    let projected = node
        .fanins
        .iter()
        .map(|fanin| value_for_fanin(*fanin, fanins, assignment))
        .collect::<CollapseResult<Vec<_>>>()?;
    Ok(evaluate_cover(&node.cover, &projected))
}

fn value_for_fanin(fanin: NodeId, fanins: &[NodeId], assignment: &[bool]) -> CollapseResult<bool> {
    fanins
        .iter()
        .position(|candidate| *candidate == fanin)
        .map(|index| assignment[index])
        .ok_or(CollapseError::UnknownFanin(fanin))
}

fn evaluate_cover(cover: &[Cube], assignment: &[bool]) -> bool {
    cover.iter().any(|cube| cube.matches_assignment(assignment))
}

fn contain(mut cover: Vec<Cube>) -> Vec<Cube> {
    let mut result = Vec::new();
    while let Some(candidate) = cover.pop() {
        if cover.iter().any(|other| other.is_superset_of(&candidate))
            || result
                .iter()
                .any(|other: &Cube| other.is_superset_of(&candidate))
        {
            continue;
        }
        result.retain(|other: &Cube| !candidate.is_superset_of(other));
        result.push(candidate);
    }
    result.reverse();
    result
}

fn minimize_cover(mut cover: Vec<Cube>) -> Vec<Cube> {
    cover = contain(cover);
    loop {
        let Some((left, right, merged)) = find_merge(&cover) else {
            return contain(cover);
        };

        let mut next = Vec::with_capacity(cover.len() - 1);
        for (index, cube) in cover.into_iter().enumerate() {
            if index != left && index != right {
                next.push(cube);
            }
        }
        next.push(merged);
        cover = contain(next);
    }
}

fn find_merge(cover: &[Cube]) -> Option<(usize, usize, Cube)> {
    for left in 0..cover.len() {
        for right in (left + 1)..cover.len() {
            if let Some(merged) = merge_pair(&cover[left], &cover[right]) {
                return Some((left, right, merged));
            }
        }
    }
    None
}

fn merge_pair(left: &Cube, right: &Cube) -> Option<Cube> {
    let mut diff = None;
    let mut literals = Vec::with_capacity(left.literals.len());
    for (index, (left_literal, right_literal)) in
        left.literals.iter().zip(&right.literals).enumerate()
    {
        if left_literal == right_literal {
            literals.push(*left_literal);
            continue;
        }
        if *left_literal == Literal::DontCare || *right_literal == Literal::DontCare {
            return None;
        }
        if diff.is_some() {
            return None;
        }
        diff = Some(index);
        literals.push(Literal::DontCare);
    }

    diff.map(|_| Cube::new(literals))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(literals: &[Literal]) -> Cube {
        Cube::new(literals.to_vec())
    }

    fn internal(id: usize, fanins: &[usize], cover: Vec<Cube>) -> CoverNode {
        CoverNode::internal(
            NodeId(id),
            format!("n{id}"),
            fanins.iter().copied().map(NodeId).collect(),
            cover,
        )
        .unwrap()
    }

    fn value(node: &CoverNode, values: &[(usize, bool)]) -> bool {
        let fanins = values.iter().map(|(id, _)| NodeId(*id)).collect::<Vec<_>>();
        let assignment = values.iter().map(|(_, value)| *value).collect::<Vec<_>>();
        evaluate_node(node, &fanins, &assignment).unwrap()
    }

    #[test]
    fn returns_false_for_boundary_nodes_and_missing_fanin() {
        let mut input = CoverNode::primary_input(NodeId(1), "a");
        let constant = CoverNode::constant(NodeId(2), true);
        assert_eq!(collapse_node(&mut input, &constant), Ok(false));

        let mut fanout = internal(3, &[1], vec![cube(&[Literal::One])]);
        assert_eq!(collapse_node(&mut fanout, &constant), Ok(false));
    }

    #[test]
    fn constant_zero_removes_cubes_requiring_one() {
        let mut fanout = internal(
            10,
            &[1, 2],
            vec![
                cube(&[Literal::One, Literal::One]),
                cube(&[Literal::Zero, Literal::One]),
                cube(&[Literal::DontCare, Literal::Zero]),
            ],
        );
        let constant = CoverNode::constant(NodeId(1), false);

        assert_eq!(collapse_node(&mut fanout, &constant), Ok(true));

        assert_eq!(fanout.fanins, vec![NodeId(2)]);
        assert_eq!(fanout.cover, vec![cube(&[Literal::DontCare])]);
    }

    #[test]
    fn constant_one_removes_cubes_requiring_zero() {
        let mut fanout = internal(
            10,
            &[1, 2],
            vec![
                cube(&[Literal::Zero, Literal::One]),
                cube(&[Literal::One, Literal::Zero]),
            ],
        );
        let constant = CoverNode::constant(NodeId(1), true);

        assert_eq!(collapse_node(&mut fanout, &constant), Ok(true));

        assert_eq!(fanout.fanins, vec![NodeId(2)]);
        assert_eq!(fanout.cover, vec![cube(&[Literal::Zero])]);
    }

    #[test]
    fn general_collapse_substitutes_buffer_logic() {
        let mut fanout = internal(
            10,
            &[7, 2],
            vec![
                cube(&[Literal::One, Literal::Zero]),
                cube(&[Literal::Zero, Literal::One]),
            ],
        );
        let collapsed = internal(7, &[1], vec![cube(&[Literal::One])]);

        assert_eq!(collapse_node(&mut fanout, &collapsed), Ok(true));

        assert_eq!(fanout.fanins, vec![NodeId(1), NodeId(2)]);
        assert_eq!(value(&fanout, &[(1, false), (2, false)]), false);
        assert_eq!(value(&fanout, &[(1, false), (2, true)]), true);
        assert_eq!(value(&fanout, &[(1, true), (2, false)]), true);
        assert_eq!(value(&fanout, &[(1, true), (2, true)]), false);
    }

    #[test]
    fn general_collapse_substitutes_multi_input_logic() {
        let mut fanout = internal(20, &[7, 3], vec![cube(&[Literal::One, Literal::One])]);
        let collapsed = internal(7, &[1, 2], vec![cube(&[Literal::One, Literal::One])]);

        assert_eq!(collapse_node(&mut fanout, &collapsed), Ok(true));

        assert_eq!(fanout.fanins, vec![NodeId(1), NodeId(2), NodeId(3)]);
        assert_eq!(value(&fanout, &[(1, true), (2, true), (3, true)]), true);
        assert_eq!(value(&fanout, &[(1, true), (2, false), (3, true)]), false);
        assert_eq!(value(&fanout, &[(1, true), (2, true), (3, false)]), false);
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let text = include_str!("collapse.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
    }
}
