//! Native sum-of-products cofactoring for SIS-style node covers.
//!
//! The legacy implementation operated on `node_t` and Espresso bit-set covers.
//! This port keeps the same Boolean behavior over owned Rust data structures:
//! algebraic cofactors split a cover by a selected fanin, and Boolean
//! cofactors by a single cube retain compatible cubes while removing the
//! cube's constrained literals.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralPhase {
    Zero,
    One,
    DontCare,
}

impl LiteralPhase {
    pub const fn is_care(self) -> bool {
        !matches!(self, Self::DontCare)
    }

    pub const fn complement(self) -> CofactorResult<Self> {
        match self {
            Self::Zero => Ok(Self::One),
            Self::One => Ok(Self::Zero),
            Self::DontCare => Err(CofactorError::InvalidCube),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    phases: Vec<LiteralPhase>,
}

impl Cube {
    pub fn new(phases: Vec<LiteralPhase>) -> Self {
        Self { phases }
    }

    pub fn tautology(width: usize) -> Self {
        Self {
            phases: vec![LiteralPhase::DontCare; width],
        }
    }

    pub fn literal(width: usize, index: usize, phase: LiteralPhase) -> CofactorResult<Self> {
        if index >= width || !phase.is_care() {
            return Err(CofactorError::InvalidFaninIndex { index, width });
        }

        let mut cube = Self::tautology(width);
        cube.phases[index] = phase;
        Ok(cube)
    }

    pub fn phases(&self) -> &[LiteralPhase] {
        &self.phases
    }

    pub fn width(&self) -> usize {
        self.phases.len()
    }

    pub fn phase(&self, index: usize) -> CofactorResult<LiteralPhase> {
        self.phases
            .get(index)
            .copied()
            .ok_or(CofactorError::InvalidFaninIndex {
                index,
                width: self.width(),
            })
    }

    pub fn with_literal_removed(&self, index: usize) -> CofactorResult<Self> {
        let mut phases = self.phases.clone();
        let width = phases.len();
        let phase = phases
            .get_mut(index)
            .ok_or(CofactorError::InvalidFaninIndex { index, width })?;
        *phase = LiteralPhase::DontCare;
        Ok(Self { phases })
    }

    pub fn without_constrained_literals(&self, condition: &Self) -> CofactorResult<Self> {
        self.ensure_same_width(condition)?;

        let phases = self
            .phases
            .iter()
            .zip(&condition.phases)
            .map(|(phase, condition_phase)| {
                if condition_phase.is_care() {
                    LiteralPhase::DontCare
                } else {
                    *phase
                }
            })
            .collect();
        Ok(Self { phases })
    }

    pub fn is_compatible_with(&self, other: &Self) -> CofactorResult<bool> {
        self.ensure_same_width(other)?;

        Ok(self
            .phases
            .iter()
            .zip(&other.phases)
            .all(|(left, right)| match (left, right) {
                (LiteralPhase::DontCare, _) | (_, LiteralPhase::DontCare) => true,
                _ => left == right,
            }))
    }

    pub fn literal_count(&self) -> usize {
        self.phases.iter().filter(|phase| phase.is_care()).count()
    }

    fn ensure_same_width(&self, other: &Self) -> CofactorResult<()> {
        if self.width() == other.width() {
            Ok(())
        } else {
            Err(CofactorError::MismatchedWidth {
                left: self.width(),
                right: other.width(),
            })
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeCover {
    fanins: Vec<NodeId>,
    cubes: Vec<Cube>,
    is_dup_free: bool,
}

impl NodeCover {
    pub fn new(fanins: Vec<NodeId>, cubes: Vec<Cube>) -> CofactorResult<Self> {
        let mut cover = Self {
            fanins,
            cubes,
            is_dup_free: false,
        };
        cover.validate()?;
        cover.minimize_base();
        Ok(cover)
    }

    pub fn with_dup_free(mut self, is_dup_free: bool) -> Self {
        self.is_dup_free = is_dup_free;
        self
    }

    pub fn constant(value: bool) -> Self {
        if value {
            Self {
                fanins: Vec::new(),
                cubes: vec![Cube::new(Vec::new())],
                is_dup_free: true,
            }
        } else {
            Self {
                fanins: Vec::new(),
                cubes: Vec::new(),
                is_dup_free: true,
            }
        }
    }

    pub fn literal(fanin: NodeId, phase: LiteralPhase) -> CofactorResult<Self> {
        if !phase.is_care() {
            return Err(CofactorError::InvalidCube);
        }

        Ok(Self {
            fanins: vec![fanin],
            cubes: vec![Cube::new(vec![phase])],
            is_dup_free: true,
        })
    }

    pub fn fanins(&self) -> &[NodeId] {
        &self.fanins
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    pub fn is_dup_free(&self) -> bool {
        self.is_dup_free
    }

    pub fn function(&self) -> NodeFunction {
        if self.cubes.is_empty() {
            return NodeFunction::Zero;
        }

        if self.fanins.is_empty() && self.cubes.len() == 1 && self.cubes[0].literal_count() == 0 {
            return NodeFunction::One;
        }

        if self.cubes.len() == 1 && self.cubes[0].literal_count() == 1 {
            let phase = self.cubes[0]
                .phases()
                .iter()
                .copied()
                .find(|phase| phase.is_care())
                .expect("literal count was checked");
            return match phase {
                LiteralPhase::One => NodeFunction::Buffer,
                LiteralPhase::Zero => NodeFunction::Inverter,
                LiteralPhase::DontCare => NodeFunction::Complex,
            };
        }

        if self.cubes.len() == 1 {
            NodeFunction::And
        } else {
            NodeFunction::Complex
        }
    }

    pub fn fanin_index(&self, fanin: NodeId) -> Option<usize> {
        self.fanins.iter().position(|candidate| *candidate == fanin)
    }

    fn validate(&self) -> CofactorResult<()> {
        let width = self.fanins.len();
        if let Some(cube) = self.cubes.iter().find(|cube| cube.width() != width) {
            return Err(CofactorError::MismatchedWidth {
                left: width,
                right: cube.width(),
            });
        }

        let unique = self.fanins.iter().copied().collect::<BTreeSet<_>>();
        if unique.len() == self.fanins.len() {
            Ok(())
        } else {
            Err(CofactorError::DuplicateFanin)
        }
    }

    fn minimize_base(&mut self) {
        if self.cubes.is_empty() {
            self.fanins.clear();
            return;
        }

        let used = (0..self.fanins.len())
            .filter(|index| self.cubes.iter().any(|cube| cube.phases[*index].is_care()))
            .collect::<BTreeSet<_>>();

        if used.len() == self.fanins.len() {
            return;
        }

        self.fanins = self
            .fanins
            .iter()
            .enumerate()
            .filter_map(|(index, fanin)| used.contains(&index).then_some(*fanin))
            .collect();

        for cube in &mut self.cubes {
            cube.phases = cube
                .phases
                .iter()
                .enumerate()
                .filter_map(|(index, phase)| used.contains(&index).then_some(*phase))
                .collect();
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Zero,
    One,
    Buffer,
    Inverter,
    And,
    Complex,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlgebraicCofactors {
    pub positive: NodeCover,
    pub negative: NodeCover,
    pub remainder: NodeCover,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CofactorError {
    ZeroCondition,
    NonCubeCondition,
    InvalidCube,
    InvalidFaninIndex { index: usize, width: usize },
    DuplicateFanin,
    MismatchedWidth { left: usize, right: usize },
}

impl fmt::Display for CofactorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCondition => write!(f, "cofactor is not defined for a zero condition"),
            Self::NonCubeCondition => write!(f, "cofactor condition must be a single cube"),
            Self::InvalidCube => write!(f, "invalid cube"),
            Self::InvalidFaninIndex { index, width } => {
                write!(f, "fanin index {index} is outside cover width {width}")
            }
            Self::DuplicateFanin => write!(f, "cover contains duplicate fanins"),
            Self::MismatchedWidth { left, right } => {
                write!(f, "cover widths differ: {left} and {right}")
            }
        }
    }
}

impl Error for CofactorError {}

pub type CofactorResult<T> = Result<T, CofactorError>;

pub fn algebraic_cofactor(node: &NodeCover, fanin: NodeId) -> CofactorResult<AlgebraicCofactors> {
    let Some(index) = node.fanin_index(fanin) else {
        return Ok(AlgebraicCofactors {
            positive: NodeCover::new(node.fanins.clone(), Vec::new())?
                .with_dup_free(node.is_dup_free),
            negative: NodeCover::new(node.fanins.clone(), Vec::new())?
                .with_dup_free(node.is_dup_free),
            remainder: node.clone(),
        });
    };

    let mut positive = Vec::new();
    let mut negative = Vec::new();
    let mut remainder = Vec::new();

    for cube in &node.cubes {
        match cube.phase(index)? {
            LiteralPhase::One => positive.push(cube.with_literal_removed(index)?),
            LiteralPhase::Zero => negative.push(cube.with_literal_removed(index)?),
            LiteralPhase::DontCare => remainder.push(cube.clone()),
        }
    }

    Ok(AlgebraicCofactors {
        positive: NodeCover::new(node.fanins.clone(), positive)?.with_dup_free(node.is_dup_free),
        negative: NodeCover::new(node.fanins.clone(), negative)?.with_dup_free(node.is_dup_free),
        remainder: NodeCover::new(node.fanins.clone(), remainder)?.with_dup_free(node.is_dup_free),
    })
}

pub fn cofactor_by_literal(
    node: &NodeCover,
    fanin: NodeId,
    phase: LiteralPhase,
) -> CofactorResult<NodeCover> {
    if !phase.is_care() {
        return Err(CofactorError::InvalidCube);
    }

    let Some(index) = node.fanin_index(fanin) else {
        return Ok(node.clone());
    };

    let mut cubes = Vec::new();
    for cube in &node.cubes {
        match cube.phase(index)? {
            cube_phase if cube_phase == phase => cubes.push(cube.with_literal_removed(index)?),
            LiteralPhase::DontCare => cubes.push(cube.clone()),
            _ => {}
        }
    }

    NodeCover::new(node.fanins.clone(), cubes).map(|cover| cover.with_dup_free(node.is_dup_free))
}

pub fn cofactor(node: &NodeCover, condition: &NodeCover) -> CofactorResult<NodeCover> {
    match condition.function() {
        NodeFunction::Zero => Err(CofactorError::ZeroCondition),
        NodeFunction::One => Ok(node.clone()),
        NodeFunction::Buffer => {
            let (fanin, phase) = only_literal(condition)?;
            cofactor_by_literal(node, fanin, phase)
        }
        NodeFunction::Inverter => {
            let (fanin, phase) = only_literal(condition)?;
            cofactor_by_literal(node, fanin, phase)
        }
        NodeFunction::And => cofactor_by_cube(node, condition),
        NodeFunction::Complex => Err(CofactorError::NonCubeCondition),
    }
}

fn cofactor_by_cube(node: &NodeCover, condition: &NodeCover) -> CofactorResult<NodeCover> {
    let (fanins, node_cube_map, condition_cube) = align_condition(node, condition)?;
    let mut cubes = Vec::new();

    for cube in &node.cubes {
        let aligned_cube = align_cube(cube, &node_cube_map, fanins.len());
        if aligned_cube.is_compatible_with(&condition_cube)? {
            cubes.push(aligned_cube.without_constrained_literals(&condition_cube)?);
        }
    }

    NodeCover::new(fanins, cubes).map(|cover| cover.with_dup_free(true))
}

fn only_literal(condition: &NodeCover) -> CofactorResult<(NodeId, LiteralPhase)> {
    let cube = condition
        .cubes
        .first()
        .ok_or(CofactorError::NonCubeCondition)?;

    let (index, phase) = cube
        .phases()
        .iter()
        .copied()
        .enumerate()
        .find(|(_, phase)| phase.is_care())
        .ok_or(CofactorError::NonCubeCondition)?;

    Ok((condition.fanins[index], phase))
}

fn align_condition(
    node: &NodeCover,
    condition: &NodeCover,
) -> CofactorResult<(Vec<NodeId>, Vec<usize>, Cube)> {
    let condition_cube = condition
        .cubes
        .first()
        .ok_or(CofactorError::NonCubeCondition)?;
    let mut fanins = node.fanins.clone();

    for fanin in &condition.fanins {
        if !fanins.contains(fanin) {
            fanins.push(*fanin);
        }
    }

    let node_cube_map = node
        .fanins
        .iter()
        .map(|fanin| {
            fanins
                .iter()
                .position(|candidate| candidate == fanin)
                .expect("node fanin was inserted")
        })
        .collect();

    let condition_map = condition
        .fanins
        .iter()
        .map(|fanin| {
            fanins
                .iter()
                .position(|candidate| candidate == fanin)
                .expect("condition fanin was inserted")
        })
        .collect::<Vec<_>>();

    let width = fanins.len();
    Ok((
        fanins,
        node_cube_map,
        align_cube(condition_cube, &condition_map, width),
    ))
}

fn align_cube(cube: &Cube, map: &[usize], width: usize) -> Cube {
    let mut phases = vec![LiteralPhase::DontCare; width];
    for (old_index, new_index) in map.iter().copied().enumerate() {
        phases[new_index] = cube.phases[old_index];
    }
    Cube::new(phases)
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: NodeId = NodeId(0);
    const B: NodeId = NodeId(1);
    const C: NodeId = NodeId(2);

    fn cover(cubes: Vec<Vec<LiteralPhase>>) -> NodeCover {
        NodeCover::new(
            vec![A, B, C],
            cubes.into_iter().map(Cube::new).collect::<Vec<_>>(),
        )
        .unwrap()
        .with_dup_free(true)
    }

    #[test]
    fn algebraic_cofactor_splits_positive_negative_and_remainder_cubes() {
        let node = cover(vec![
            vec![
                LiteralPhase::One,
                LiteralPhase::Zero,
                LiteralPhase::DontCare,
            ],
            vec![
                LiteralPhase::Zero,
                LiteralPhase::One,
                LiteralPhase::DontCare,
            ],
            vec![
                LiteralPhase::DontCare,
                LiteralPhase::One,
                LiteralPhase::Zero,
            ],
        ]);

        let cofactors = algebraic_cofactor(&node, A).unwrap();

        assert_eq!(
            cofactors.positive.cubes(),
            &[Cube::new(vec![LiteralPhase::Zero])]
        );
        assert_eq!(
            cofactors.negative.cubes(),
            &[Cube::new(vec![LiteralPhase::One])]
        );
        assert_eq!(
            cofactors.remainder.cubes(),
            &[Cube::new(vec![LiteralPhase::One, LiteralPhase::Zero])]
        );
        assert!(cofactors.positive.is_dup_free());
        assert!(cofactors.negative.is_dup_free());
        assert!(cofactors.remainder.is_dup_free());
    }

    #[test]
    fn algebraic_cofactor_for_absent_fanin_preserves_remainder() {
        let node = cover(vec![vec![
            LiteralPhase::One,
            LiteralPhase::DontCare,
            LiteralPhase::Zero,
        ]]);

        let cofactors = algebraic_cofactor(&node, NodeId(99)).unwrap();

        assert!(cofactors.positive.cubes().is_empty());
        assert!(cofactors.negative.cubes().is_empty());
        assert_eq!(cofactors.remainder, node);
    }

    #[test]
    fn literal_cofactor_keeps_matching_and_dont_care_cubes() {
        let node = cover(vec![
            vec![
                LiteralPhase::One,
                LiteralPhase::Zero,
                LiteralPhase::DontCare,
            ],
            vec![
                LiteralPhase::Zero,
                LiteralPhase::One,
                LiteralPhase::DontCare,
            ],
            vec![
                LiteralPhase::DontCare,
                LiteralPhase::One,
                LiteralPhase::Zero,
            ],
        ]);

        let result = cofactor_by_literal(&node, A, LiteralPhase::One).unwrap();

        assert_eq!(result.fanins(), &[B, C]);
        assert_eq!(
            result.cubes(),
            &[
                Cube::new(vec![LiteralPhase::Zero, LiteralPhase::DontCare]),
                Cube::new(vec![LiteralPhase::One, LiteralPhase::Zero]),
            ]
        );
    }

    #[test]
    fn cube_cofactor_retains_compatible_cubes_and_removes_condition_literals() {
        let node = cover(vec![
            vec![
                LiteralPhase::One,
                LiteralPhase::Zero,
                LiteralPhase::DontCare,
            ],
            vec![LiteralPhase::One, LiteralPhase::One, LiteralPhase::DontCare],
            vec![
                LiteralPhase::DontCare,
                LiteralPhase::Zero,
                LiteralPhase::One,
            ],
        ]);
        let condition = NodeCover::new(
            vec![A, B],
            vec![Cube::new(vec![LiteralPhase::One, LiteralPhase::Zero])],
        )
        .unwrap();

        let result = cofactor(&node, &condition).unwrap();

        assert_eq!(result.fanins(), &[C]);
        assert_eq!(
            result.cubes(),
            &[
                Cube::new(vec![LiteralPhase::DontCare]),
                Cube::new(vec![LiteralPhase::One])
            ]
        );
        assert!(result.is_dup_free());
    }

    #[test]
    fn cofactor_rejects_zero_and_non_cube_conditions() {
        let node = cover(vec![vec![
            LiteralPhase::One,
            LiteralPhase::DontCare,
            LiteralPhase::Zero,
        ]]);
        let zero = NodeCover::constant(false);
        let non_cube = NodeCover::new(
            vec![A, B],
            vec![
                Cube::new(vec![LiteralPhase::One, LiteralPhase::DontCare]),
                Cube::new(vec![LiteralPhase::DontCare, LiteralPhase::One]),
            ],
        )
        .unwrap();

        assert_eq!(cofactor(&node, &zero), Err(CofactorError::ZeroCondition));
        assert_eq!(
            cofactor(&node, &non_cube),
            Err(CofactorError::NonCubeCondition)
        );
    }

    #[test]
    fn no_legacy_c_abi_or_beads_metadata_tokens_are_present() {
        let source = include_str!("cofct.rs");

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
