//! Native algebraic substitution for SIS-style node covers.
//!
//! Substitution rewrites an internal node `f` by extracting occurrences of
//! another internal node `g` from `f`'s cover. A successful rewrite introduces
//! `g` itself as a fanin literal and preserves the original Boolean value as
//! `(f / g) * g + remainder`. When requested, the complement phase performs the
//! same operation with `!g`.

use std::collections::{BTreeMap, BTreeSet};
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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Literal {
    Zero,
    One,
    DontCare,
}

impl Literal {
    pub const fn is_care(self) -> bool {
        !matches!(self, Self::DontCare)
    }

    pub const fn matches_value(self, value: bool) -> bool {
        match self {
            Self::Zero => !value,
            Self::One => value,
            Self::DontCare => true,
        }
    }

    const fn from_value(value: bool) -> Self {
        if value { Self::One } else { Self::Zero }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cube {
    literals: Vec<Literal>,
}

impl Cube {
    pub fn new(literals: Vec<Literal>) -> Self {
        Self { literals }
    }

    pub fn tautology(width: usize) -> Self {
        Self {
            literals: vec![Literal::DontCare; width],
        }
    }

    pub fn literal(width: usize, index: usize, phase: Literal) -> SubstituteResult<Self> {
        if index >= width || !phase.is_care() {
            return Err(SubstituteError::InvalidLiteral {
                index,
                width,
                phase,
            });
        }

        let mut cube = Self::tautology(width);
        cube.literals[index] = phase;
        Ok(cube)
    }

    pub fn literals(&self) -> &[Literal] {
        &self.literals
    }

    pub fn width(&self) -> usize {
        self.literals.len()
    }

    pub fn literal_count(&self) -> usize {
        self.literals
            .iter()
            .filter(|literal| literal.is_care())
            .count()
    }

    fn covers(&self, other: &Self) -> bool {
        self.literals
            .iter()
            .zip(&other.literals)
            .all(|(left, right)| *left == Literal::DontCare || left == right)
    }

    fn intersect(&self, other: &Self) -> Option<Self> {
        let mut literals = Vec::with_capacity(self.width());
        for (left, right) in self.literals.iter().zip(&other.literals) {
            match (*left, *right) {
                (Literal::DontCare, literal) | (literal, Literal::DontCare) => {
                    literals.push(literal);
                }
                (left, right) if left == right => literals.push(left),
                _ => return None,
            }
        }

        Some(Self { literals })
    }

    fn quotient_by(&self, divisor: &Self) -> Option<Self> {
        if !self.covers_with_cares(divisor) {
            return None;
        }

        let literals = self
            .literals
            .iter()
            .zip(&divisor.literals)
            .map(|(left, right)| {
                if right.is_care() {
                    Literal::DontCare
                } else {
                    *left
                }
            })
            .collect();
        Some(Self { literals })
    }

    fn covers_with_cares(&self, divisor: &Self) -> bool {
        self.literals
            .iter()
            .zip(&divisor.literals)
            .all(|(left, right)| !right.is_care() || left == right)
    }

    fn merge_distance_one(&self, other: &Self) -> Option<Self> {
        let mut difference = None;
        let mut literals = self.literals.clone();

        for (index, (left, right)) in self.literals.iter().zip(&other.literals).enumerate() {
            if left == right {
                continue;
            }

            if left.is_care() && right.is_care() && difference.is_none() {
                difference = Some(index);
                literals[index] = Literal::DontCare;
            } else {
                return None;
            }
        }

        difference.map(|_| Self { literals })
    }

    fn matches_assignment(&self, assignment: &[bool]) -> bool {
        self.literals
            .iter()
            .zip(assignment)
            .all(|(literal, value)| literal.matches_value(*value))
    }

    fn remap(&self, old_fanins: &[NodeId], new_fanins: &[NodeId]) -> Self {
        let mut literals = vec![Literal::DontCare; new_fanins.len()];
        for (old_index, old_fanin) in old_fanins.iter().enumerate() {
            if let Some(new_index) = new_fanins
                .iter()
                .position(|new_fanin| new_fanin == old_fanin)
            {
                literals[new_index] = self.literals[old_index];
            }
        }

        Self { literals }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover {
    fanins: Vec<NodeId>,
    cubes: Vec<Cube>,
}

impl Cover {
    pub fn new(fanins: Vec<NodeId>, cubes: Vec<Cube>) -> SubstituteResult<Self> {
        let cover = Self { fanins, cubes };
        cover.validate()?;
        Ok(cover.minimized())
    }

    pub fn constant(value: bool) -> Self {
        if value {
            Self {
                fanins: Vec::new(),
                cubes: vec![Cube::new(Vec::new())],
            }
        } else {
            Self {
                fanins: Vec::new(),
                cubes: Vec::new(),
            }
        }
    }

    pub fn literal(fanin: NodeId, phase: Literal) -> SubstituteResult<Self> {
        if !phase.is_care() {
            return Err(SubstituteError::InvalidLiteral {
                index: 0,
                width: 1,
                phase,
            });
        }

        Self::new(vec![fanin], vec![Cube::new(vec![phase])])
    }

    pub fn fanins(&self) -> &[NodeId] {
        &self.fanins
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    pub fn is_zero(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn support(&self) -> BTreeSet<NodeId> {
        let mut support = BTreeSet::new();
        for cube in &self.cubes {
            for (index, literal) in cube.literals.iter().enumerate() {
                if literal.is_care() {
                    support.insert(self.fanins[index]);
                }
            }
        }

        support
    }

    pub fn evaluates(&self, assignment: &BTreeMap<NodeId, bool>) -> SubstituteResult<bool> {
        let values = self
            .fanins
            .iter()
            .map(|fanin| {
                assignment
                    .get(fanin)
                    .copied()
                    .ok_or(SubstituteError::MissingAssignment(*fanin))
            })
            .collect::<SubstituteResult<Vec<_>>>()?;

        Ok(self
            .cubes
            .iter()
            .any(|cube| cube.matches_assignment(&values)))
    }

    fn validate(&self) -> SubstituteResult<()> {
        let unique = self.fanins.iter().copied().collect::<BTreeSet<_>>();
        if unique.len() != self.fanins.len() {
            return Err(SubstituteError::DuplicateFanin);
        }

        if let Some(cube) = self
            .cubes
            .iter()
            .find(|cube| cube.width() != self.fanins.len())
        {
            return Err(SubstituteError::CubeWidthMismatch {
                expected: self.fanins.len(),
                actual: cube.width(),
            });
        }

        Ok(())
    }

    fn common_base(&self, other: &Self) -> (Vec<NodeId>, Vec<Cube>, Vec<Cube>) {
        let mut fanins = self.fanins.clone();
        for fanin in &other.fanins {
            if !fanins.contains(fanin) {
                fanins.push(*fanin);
            }
        }

        (
            fanins.clone(),
            self.cubes
                .iter()
                .map(|cube| cube.remap(&self.fanins, &fanins))
                .collect(),
            other
                .cubes
                .iter()
                .map(|cube| cube.remap(&other.fanins, &fanins))
                .collect(),
        )
    }

    fn and(&self, other: &Self) -> SubstituteResult<Self> {
        let (fanins, left, right) = self.common_base(other);
        let mut cubes = Vec::new();
        for left_cube in &left {
            for right_cube in &right {
                if let Some(cube) = left_cube.intersect(right_cube) {
                    cubes.push(cube);
                }
            }
        }

        Self::new(fanins, cubes)
    }

    fn or(&self, other: &Self) -> SubstituteResult<Self> {
        let (fanins, mut cubes, mut right) = self.common_base(other);
        cubes.append(&mut right);
        Self::new(fanins, cubes)
    }

    fn complement(&self) -> SubstituteResult<Self> {
        let mut cubes = Vec::new();
        visit_assignments(self.fanins.len(), &mut Vec::new(), &mut |assignment| {
            let covered = self
                .cubes
                .iter()
                .any(|cube| cube.matches_assignment(assignment));
            if !covered {
                cubes.push(Cube::new(
                    assignment
                        .iter()
                        .copied()
                        .map(Literal::from_value)
                        .collect(),
                ));
            }
        });

        Self::new(self.fanins.clone(), cubes)
    }

    fn divide(&self, divisor: &Self) -> SubstituteResult<Division> {
        if self.cubes.len() < divisor.cubes.len() || divisor.cubes.is_empty() {
            return Ok(Division {
                quotient: Self::constant(false),
                remainder: self.clone(),
            });
        }

        let (fanins, dividend, divisor_cubes) = self.common_base(divisor);
        if !support_from_cubes(&fanins, &divisor_cubes)
            .is_subset(&support_from_cubes(&fanins, &dividend))
        {
            return Ok(Division {
                quotient: Self::constant(false),
                remainder: self.clone(),
            });
        }

        let mut candidates: BTreeMap<Cube, Vec<Option<usize>>> = BTreeMap::new();
        for (dividend_index, dividend_cube) in dividend.iter().enumerate() {
            for (divisor_index, divisor_cube) in divisor_cubes.iter().enumerate() {
                if let Some(quotient_cube) = dividend_cube.quotient_by(divisor_cube) {
                    candidates
                        .entry(quotient_cube)
                        .or_insert_with(|| vec![None; divisor_cubes.len()])[divisor_index] =
                        Some(dividend_index);
                }
            }
        }

        let mut quotient_cubes = Vec::new();
        let mut consumed = BTreeSet::new();
        for (quotient_cube, matches) in candidates {
            if matches.iter().all(Option::is_some) {
                quotient_cubes.push(quotient_cube);
                for index in matches.into_iter().flatten() {
                    consumed.insert(index);
                }
            }
        }

        if quotient_cubes.is_empty() {
            return Ok(Division {
                quotient: Self::constant(false),
                remainder: self.clone(),
            });
        }

        let remainder_cubes = dividend
            .into_iter()
            .enumerate()
            .filter_map(|(index, cube)| (!consumed.contains(&index)).then_some(cube))
            .collect();

        Ok(Division {
            quotient: Self::new(fanins.clone(), quotient_cubes)?,
            remainder: Self::new(fanins, remainder_cubes)?,
        })
    }

    fn minimized(mut self) -> Self {
        self = self.contained();
        loop {
            let Some((left, right, merged)) = find_merge(&self.cubes) else {
                return self.minimum_base();
            };

            let mut cubes = Vec::with_capacity(self.cubes.len() - 1);
            for (index, cube) in self.cubes.into_iter().enumerate() {
                if index != left && index != right {
                    cubes.push(cube);
                }
            }
            cubes.push(merged);
            self.cubes = cubes;
            self = self.contained();
        }
    }

    fn contained(mut self) -> Self {
        let mut unique = Vec::new();
        for cube in self.cubes.drain(..) {
            if !unique.contains(&cube) {
                unique.push(cube);
            }
        }

        let mut cubes = Vec::new();
        'candidate: for (index, cube) in unique.iter().enumerate() {
            for (other_index, other) in unique.iter().enumerate() {
                if index != other_index && other.covers(cube) {
                    continue 'candidate;
                }
            }

            cubes.push(cube.clone());
        }

        Self {
            fanins: self.fanins,
            cubes,
        }
    }

    fn minimum_base(mut self) -> Self {
        if self.cubes.is_empty() {
            self.fanins.clear();
            return self;
        }

        let used = (0..self.fanins.len())
            .filter(|index| {
                self.cubes
                    .iter()
                    .any(|cube| cube.literals[*index].is_care())
            })
            .collect::<BTreeSet<_>>();

        if used.len() == self.fanins.len() {
            return self;
        }

        self.fanins = self
            .fanins
            .iter()
            .enumerate()
            .filter_map(|(index, fanin)| used.contains(&index).then_some(*fanin))
            .collect();

        for cube in &mut self.cubes {
            cube.literals = cube
                .literals
                .iter()
                .enumerate()
                .filter_map(|(index, literal)| used.contains(&index).then_some(*literal))
                .collect();
        }

        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubstituteNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub cover: Option<Cover>,
}

impl SubstituteNode {
    pub fn internal(id: NodeId, cover: Cover) -> Self {
        Self {
            id,
            kind: NodeKind::Internal,
            cover: Some(cover),
        }
    }

    pub fn primary_input(id: NodeId) -> Self {
        Self {
            id,
            kind: NodeKind::PrimaryInput,
            cover: None,
        }
    }

    pub fn primary_output(id: NodeId) -> Self {
        Self {
            id,
            kind: NodeKind::PrimaryOutput,
            cover: None,
        }
    }

    pub fn cover(&self) -> Option<&Cover> {
        self.cover.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubstituteError {
    MissingFunction {
        node: NodeId,
    },
    DuplicateFanin,
    CubeWidthMismatch {
        expected: usize,
        actual: usize,
    },
    InvalidLiteral {
        index: usize,
        width: usize,
        phase: Literal,
    },
    MissingAssignment(NodeId),
}

impl fmt::Display for SubstituteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFunction { node } => {
                write!(f, "node {:?} does not have a Boolean function", node)
            }
            Self::DuplicateFanin => write!(f, "cover contains duplicate fanins"),
            Self::CubeWidthMismatch { expected, actual } => {
                write!(f, "cube has {actual} literals, expected {expected}")
            }
            Self::InvalidLiteral {
                index,
                width,
                phase,
            } => write!(
                f,
                "literal {:?} at index {index} is invalid for cover width {width}",
                phase
            ),
            Self::MissingAssignment(node) => write!(f, "missing assignment for {:?}", node),
        }
    }
}

impl Error for SubstituteError {}

pub type SubstituteResult<T> = Result<T, SubstituteError>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Division {
    quotient: Cover,
    remainder: Cover,
}

pub fn substitute_node(
    f: &mut SubstituteNode,
    g: &SubstituteNode,
    use_complement: bool,
) -> SubstituteResult<bool> {
    if f.kind != NodeKind::Internal || g.kind != NodeKind::Internal || f.id == g.id {
        return Ok(false);
    }

    let f_cover = f
        .cover
        .as_ref()
        .ok_or(SubstituteError::MissingFunction { node: f.id })?;
    let g_cover = g
        .cover
        .as_ref()
        .ok_or(SubstituteError::MissingFunction { node: g.id })?;

    if !g_cover.support().is_subset(&f_cover.support()) {
        return Ok(false);
    }

    let (mut changed, mut rewritten) = divide_one_phase(f_cover, g.id, g_cover, Literal::One)?;
    if use_complement {
        let (changed_complement, complement_rewritten) =
            divide_one_phase(&rewritten, g.id, g_cover, Literal::Zero)?;
        changed |= changed_complement;
        rewritten = complement_rewritten;
    }

    if changed {
        f.cover = Some(rewritten);
    }

    Ok(changed)
}

fn divide_one_phase(
    f: &Cover,
    divisor_node: NodeId,
    divisor: &Cover,
    phase: Literal,
) -> SubstituteResult<(bool, Cover)> {
    let divisor_cover = if phase == Literal::Zero {
        divisor.complement()?
    } else {
        divisor.clone()
    };

    let division = f.divide(&divisor_cover)?;
    if division.quotient.is_zero() {
        return Ok((false, division.remainder));
    }

    let literal = Cover::literal(divisor_node, phase)?;
    let product = division.quotient.and(&literal)?;
    Ok((true, product.or(&division.remainder)?))
}

fn support_from_cubes(fanins: &[NodeId], cubes: &[Cube]) -> BTreeSet<NodeId> {
    let mut support = BTreeSet::new();
    for cube in cubes {
        for (index, literal) in cube.literals.iter().enumerate() {
            if literal.is_care() {
                support.insert(fanins[index]);
            }
        }
    }

    support
}

fn find_merge(cubes: &[Cube]) -> Option<(usize, usize, Cube)> {
    for left in 0..cubes.len() {
        for right in (left + 1)..cubes.len() {
            if let Some(merged) = cubes[left].merge_distance_one(&cubes[right]) {
                return Some((left, right, merged));
            }
        }
    }

    None
}

fn visit_assignments<F>(width: usize, partial: &mut Vec<bool>, visit: &mut F)
where
    F: FnMut(&[bool]),
{
    if partial.len() == width {
        visit(partial);
        return;
    }

    partial.push(false);
    visit_assignments(width, partial, visit);
    partial.pop();
    partial.push(true);
    visit_assignments(width, partial, visit);
    partial.pop();
}

#[cfg(test)]
mod tests {
    use super::*;

    const F: NodeId = NodeId(10);
    const G: NodeId = NodeId(20);
    const A: NodeId = NodeId(1);
    const B: NodeId = NodeId(2);
    const C: NodeId = NodeId(3);

    fn cube(literals: &[Literal]) -> Cube {
        Cube::new(literals.to_vec())
    }

    fn cover(fanins: &[NodeId], cubes: Vec<Cube>) -> Cover {
        Cover::new(fanins.to_vec(), cubes).unwrap()
    }

    fn assignment(values: &[(NodeId, bool)]) -> BTreeMap<NodeId, bool> {
        values.iter().copied().collect()
    }

    fn and_cover(left: NodeId, right: NodeId) -> Cover {
        cover(&[left, right], vec![cube(&[Literal::One, Literal::One])])
    }

    #[test]
    fn substitutes_positive_phase_with_node_literal() {
        let mut f = SubstituteNode::internal(
            F,
            cover(
                &[A, B, C],
                vec![
                    cube(&[Literal::One, Literal::One, Literal::One]),
                    cube(&[Literal::Zero, Literal::One, Literal::One]),
                ],
            ),
        );
        let g = SubstituteNode::internal(G, and_cover(B, C));

        assert_eq!(substitute_node(&mut f, &g, false), Ok(true));

        let result = f.cover().unwrap();
        assert_eq!(result.fanins(), &[G]);
        assert_eq!(result.cubes(), &[cube(&[Literal::One])]);
    }

    #[test]
    fn leaves_unmatched_remainder_after_substitution() {
        let mut f = SubstituteNode::internal(
            F,
            cover(
                &[A, B, C],
                vec![
                    cube(&[Literal::One, Literal::One, Literal::One]),
                    cube(&[Literal::Zero, Literal::One, Literal::One]),
                    cube(&[Literal::One, Literal::Zero, Literal::DontCare]),
                ],
            ),
        );
        let g = SubstituteNode::internal(G, and_cover(B, C));

        assert_eq!(substitute_node(&mut f, &g, false), Ok(true));

        let result = f.cover().unwrap();
        assert_eq!(result.fanins(), &[G, A, B]);
        assert_eq!(
            result.cubes(),
            &[
                cube(&[Literal::One, Literal::DontCare, Literal::DontCare]),
                cube(&[Literal::DontCare, Literal::One, Literal::Zero])
            ]
        );
    }

    #[test]
    fn optional_complement_phase_extracts_negative_literal() {
        let mut f = SubstituteNode::internal(F, cover(&[A], vec![cube(&[Literal::Zero])]));
        let g = SubstituteNode::internal(G, cover(&[A], vec![cube(&[Literal::One])]));

        assert_eq!(substitute_node(&mut f, &g, true), Ok(true));

        let result = f.cover().unwrap();
        assert_eq!(result.fanins(), &[G]);
        assert_eq!(result.cubes(), &[cube(&[Literal::Zero])]);
    }

    #[test]
    fn does_not_substitute_boundary_nodes_self_or_missing_support() {
        let mut input = SubstituteNode::primary_input(F);
        let g = SubstituteNode::internal(G, Cover::literal(A, Literal::One).unwrap());
        assert_eq!(substitute_node(&mut input, &g, true), Ok(false));

        let mut f = SubstituteNode::internal(F, Cover::literal(A, Literal::One).unwrap());
        let output = SubstituteNode::primary_output(G);
        assert_eq!(substitute_node(&mut f, &output, true), Ok(false));

        let self_node = f.clone();
        assert_eq!(substitute_node(&mut f, &self_node, true), Ok(false));

        let missing = SubstituteNode::internal(G, Cover::literal(B, Literal::One).unwrap());
        assert_eq!(substitute_node(&mut f, &missing, true), Ok(false));
    }

    #[test]
    fn reports_missing_function_for_internal_nodes() {
        let mut f = SubstituteNode {
            id: F,
            kind: NodeKind::Internal,
            cover: None,
        };
        let g = SubstituteNode::internal(G, Cover::literal(A, Literal::One).unwrap());

        assert_eq!(
            substitute_node(&mut f, &g, true),
            Err(SubstituteError::MissingFunction { node: F })
        );
    }

    #[test]
    fn rewritten_cover_matches_original_when_new_fanin_matches_divisor() {
        let original = cover(
            &[A, B, C],
            vec![
                cube(&[Literal::One, Literal::One, Literal::One]),
                cube(&[Literal::Zero, Literal::One, Literal::One]),
                cube(&[Literal::Zero, Literal::Zero, Literal::One]),
            ],
        );
        let mut f = SubstituteNode::internal(F, original.clone());
        let g_cover = and_cover(B, C);
        let g = SubstituteNode::internal(G, g_cover.clone());

        assert_eq!(substitute_node(&mut f, &g, false), Ok(true));

        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    let original_assignment = assignment(&[(A, a), (B, b), (C, c)]);
                    let g_value = g_cover.evaluates(&original_assignment).unwrap();
                    let result_assignment = assignment(&[(A, a), (B, b), (C, c), (G, g_value)]);

                    assert_eq!(
                        original.evaluates(&original_assignment).unwrap(),
                        f.cover().unwrap().evaluates(&result_assignment).unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let source = include_str!("substitute.rs");

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
