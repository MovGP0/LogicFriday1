//! Native algebraic division for SIS-style sum-of-products node covers.
//!
//! The port keeps the legacy operation in owned Rust data: divide a dividend
//! cover by a divisor cover, return the algebraic quotient, and optionally
//! return the dividend cubes that were not consumed by the division.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LiteralPhase {
    Zero,
    One,
    DontCare,
}

impl LiteralPhase {
    pub const fn is_care(self) -> bool {
        !matches!(self, Self::DontCare)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
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

    pub fn phases(&self) -> &[LiteralPhase] {
        &self.phases
    }

    pub fn width(&self) -> usize {
        self.phases.len()
    }

    pub fn literal_count(&self) -> usize {
        self.phases.iter().filter(|phase| phase.is_care()).count()
    }

    fn divides(&self, dividend: &Self) -> DivideResult<bool> {
        self.ensure_same_width(dividend)?;

        Ok(self
            .phases
            .iter()
            .zip(&dividend.phases)
            .all(|(divisor, dividend)| !divisor.is_care() || divisor == dividend))
    }

    fn quotient_after_dividing_by(&self, divisor: &Self) -> DivideResult<Self> {
        self.ensure_same_width(divisor)?;

        let phases = self
            .phases
            .iter()
            .zip(&divisor.phases)
            .map(|(dividend, divisor)| {
                if divisor.is_care() {
                    LiteralPhase::DontCare
                } else {
                    *dividend
                }
            })
            .collect();

        Ok(Self { phases })
    }

    fn covers(&self, other: &Self) -> bool {
        self.phases
            .iter()
            .zip(&other.phases)
            .all(|(left, right)| !left.is_care() || left == right)
    }

    fn ensure_same_width(&self, other: &Self) -> DivideResult<()> {
        if self.width() == other.width() {
            Ok(())
        } else {
            Err(DivideError::MismatchedWidth {
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
    is_scc_minimal: bool,
}

impl NodeCover {
    pub fn new(fanins: Vec<NodeId>, cubes: Vec<Cube>) -> DivideResult<Self> {
        let mut cover = Self {
            fanins,
            cubes,
            is_dup_free: false,
            is_scc_minimal: false,
        };
        cover.validate()?;
        cover.contain();
        cover.minimize_base();
        Ok(cover)
    }

    pub fn constant(value: bool) -> Self {
        Self {
            fanins: Vec::new(),
            cubes: value.then(|| Cube::new(Vec::new())).into_iter().collect(),
            is_dup_free: true,
            is_scc_minimal: true,
        }
    }

    pub fn literal(fanin: NodeId, phase: LiteralPhase) -> DivideResult<Self> {
        if !phase.is_care() {
            return Err(DivideError::InvalidLiteral);
        }

        Ok(Self {
            fanins: vec![fanin],
            cubes: vec![Cube::new(vec![phase])],
            is_dup_free: true,
            is_scc_minimal: true,
        })
    }

    pub fn with_flags(mut self, is_dup_free: bool, is_scc_minimal: bool) -> Self {
        self.is_dup_free = is_dup_free;
        self.is_scc_minimal = is_scc_minimal;
        self
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

    pub fn is_scc_minimal(&self) -> bool {
        self.is_scc_minimal
    }

    pub fn is_zero(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn is_one(&self) -> bool {
        self.fanins.is_empty() && self.cubes.len() == 1 && self.cubes[0].literal_count() == 0
    }

    pub fn fanin_index(&self, fanin: NodeId) -> Option<usize> {
        self.fanins.iter().position(|candidate| *candidate == fanin)
    }

    fn validate(&self) -> DivideResult<()> {
        let width = self.fanins.len();
        if let Some(cube) = self.cubes.iter().find(|cube| cube.width() != width) {
            return Err(DivideError::MismatchedWidth {
                left: width,
                right: cube.width(),
            });
        }

        let unique = self.fanins.iter().copied().collect::<BTreeSet<_>>();
        if unique.len() == self.fanins.len() {
            Ok(())
        } else {
            Err(DivideError::DuplicateFanin)
        }
    }

    fn remap(&self, fanins: &[NodeId]) -> Self {
        let map = self
            .fanins
            .iter()
            .map(|fanin| {
                fanins
                    .iter()
                    .position(|candidate| candidate == fanin)
                    .expect("fanin was inserted into common base")
            })
            .collect::<Vec<_>>();

        Self {
            fanins: fanins.to_vec(),
            cubes: self
                .cubes
                .iter()
                .map(|cube| remap_cube(cube, &map, fanins.len()))
                .collect(),
            is_dup_free: self.is_dup_free,
            is_scc_minimal: self.is_scc_minimal,
        }
    }

    fn support(&self) -> BTreeSet<NodeId> {
        let mut support = BTreeSet::new();
        for cube in &self.cubes {
            for (index, phase) in cube.phases.iter().copied().enumerate() {
                if phase.is_care() {
                    support.insert(self.fanins[index]);
                }
            }
        }

        support
    }

    fn contain(&mut self) {
        let mut unique = Vec::new();
        for cube in self.cubes.drain(..) {
            if !unique.contains(&cube) {
                unique.push(cube);
            }
        }

        let mut reduced = Vec::new();
        'candidate: for (index, cube) in unique.iter().enumerate() {
            for (other_index, other) in unique.iter().enumerate() {
                if index != other_index && other.covers(cube) {
                    continue 'candidate;
                }
            }

            reduced.push(cube.clone());
        }

        self.cubes = reduced;
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Division {
    pub quotient: NodeCover,
    pub remainder: Option<NodeCover>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DivideError {
    DivisionByZero,
    DuplicateFanin,
    InvalidLiteral,
    MissingFunction,
    MismatchedWidth { left: usize, right: usize },
}

impl fmt::Display for DivideError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DivisionByZero => write!(f, "cannot divide by a zero cover"),
            Self::DuplicateFanin => write!(f, "cover contains duplicate fanins"),
            Self::InvalidLiteral => write!(f, "literal phase must be zero or one"),
            Self::MissingFunction => write!(f, "division requires covers with Boolean functions"),
            Self::MismatchedWidth { left, right } => {
                write!(f, "cover widths differ: {left} and {right}")
            }
        }
    }
}

impl Error for DivideError {}

pub type DivideResult<T> = Result<T, DivideError>;

pub fn divide(
    dividend: &NodeCover,
    divisor: &NodeCover,
    include_remainder: bool,
) -> DivideResult<Division> {
    if divisor.is_zero() {
        return Err(DivideError::DivisionByZero);
    }

    if dividend.cubes.len() < divisor.cubes.len() {
        return trivial_division(dividend, include_remainder);
    }

    if divisor.cubes.len() == 1 {
        if divisor.fanins.len() == 1 {
            return divide_single_literal(dividend, divisor, include_remainder);
        }

        return divide_single_cube(dividend, divisor, include_remainder);
    }

    divide_multiple_cubes(dividend, divisor, include_remainder)
}

fn divide_single_literal(
    dividend: &NodeCover,
    divisor: &NodeCover,
    include_remainder: bool,
) -> DivideResult<Division> {
    let Some(index) = dividend.fanin_index(divisor.fanins[0]) else {
        return trivial_division(dividend, include_remainder);
    };

    let divisor_phase = divisor.cubes[0].phases[0];
    let mut quotient = Vec::new();
    let mut remainder = Vec::new();

    for cube in &dividend.cubes {
        if cube.phases[index] == divisor_phase {
            let mut quotient_cube = cube.clone();
            quotient_cube.phases[index] = LiteralPhase::DontCare;
            quotient.push(quotient_cube);
        } else {
            remainder.push(cube.clone());
        }
    }

    make_division(
        dividend.fanins.clone(),
        quotient,
        include_remainder.then_some((dividend.fanins.clone(), remainder)),
        dividend,
    )
}

fn divide_single_cube(
    dividend: &NodeCover,
    divisor: &NodeCover,
    include_remainder: bool,
) -> DivideResult<Division> {
    let (fanins, dividend, divisor) = common_base(dividend, divisor);
    let divisor_cube = &divisor.cubes[0];
    let mut quotient = Vec::new();
    let mut remainder = Vec::new();

    for cube in &dividend.cubes {
        if divisor_cube.divides(cube)? {
            quotient.push(cube.quotient_after_dividing_by(divisor_cube)?);
        } else {
            remainder.push(cube.clone());
        }
    }

    make_division(
        fanins.clone(),
        quotient,
        include_remainder.then_some((fanins, remainder)),
        &dividend,
    )
}

fn divide_multiple_cubes(
    dividend: &NodeCover,
    divisor: &NodeCover,
    include_remainder: bool,
) -> DivideResult<Division> {
    if !dividend.support().is_superset(&divisor.support()) {
        return trivial_division(dividend, include_remainder);
    }

    let (fanins, dividend, divisor) = common_base(dividend, divisor);

    let mut candidates: BTreeMap<Cube, Vec<Cube>> = BTreeMap::new();
    for dividend_cube in &dividend.cubes {
        for divisor_cube in &divisor.cubes {
            if divisor_cube.divides(dividend_cube)? {
                let quotient_cube = dividend_cube.quotient_after_dividing_by(divisor_cube)?;
                candidates
                    .entry(quotient_cube)
                    .or_default()
                    .push(dividend_cube.clone());
            }
        }
    }

    let divisor_cube_count = divisor.cubes.len();
    let mut consumed = BTreeSet::new();
    let mut quotient = Vec::new();
    for (quotient_cube, originals) in &candidates {
        if originals.len() == divisor_cube_count {
            quotient.push(quotient_cube.clone());
            consumed.extend(originals.iter().cloned());
        }
    }

    let remainder = include_remainder.then(|| {
        dividend
            .cubes
            .iter()
            .filter(|cube| !consumed.contains(*cube))
            .cloned()
            .collect::<Vec<_>>()
    });

    make_division(
        fanins.clone(),
        quotient,
        remainder.map(|cubes| (fanins, cubes)),
        &dividend,
    )
}

fn trivial_division(dividend: &NodeCover, include_remainder: bool) -> DivideResult<Division> {
    Ok(Division {
        quotient: NodeCover::constant(false),
        remainder: include_remainder.then_some(dividend.clone()),
    })
}

fn make_division(
    quotient_fanins: Vec<NodeId>,
    quotient_cubes: Vec<Cube>,
    remainder: Option<(Vec<NodeId>, Vec<Cube>)>,
    flags: &NodeCover,
) -> DivideResult<Division> {
    let quotient =
        NodeCover::new(quotient_fanins, quotient_cubes)?.with_flags(true, flags.is_scc_minimal);
    let remainder = remainder
        .map(|(fanins, cubes)| {
            NodeCover::new(fanins, cubes).map(|cover| cover.with_flags(true, flags.is_scc_minimal))
        })
        .transpose()?;

    Ok(Division {
        quotient,
        remainder,
    })
}

fn common_base(left: &NodeCover, right: &NodeCover) -> (Vec<NodeId>, NodeCover, NodeCover) {
    let mut fanins = left.fanins.clone();
    for fanin in &right.fanins {
        if !fanins.contains(fanin) {
            fanins.push(*fanin);
        }
    }

    (fanins.clone(), left.remap(&fanins), right.remap(&fanins))
}

fn remap_cube(cube: &Cube, map: &[usize], width: usize) -> Cube {
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

    fn cube(phases: &[LiteralPhase]) -> Cube {
        Cube::new(phases.to_vec())
    }

    fn cover(fanins: &[NodeId], cubes: Vec<Cube>) -> NodeCover {
        NodeCover::new(fanins.to_vec(), cubes).unwrap()
    }

    #[test]
    fn single_literal_division_removes_matching_literal() {
        let dividend = cover(
            &[A, B],
            vec![
                cube(&[LiteralPhase::One, LiteralPhase::One]),
                cube(&[LiteralPhase::Zero, LiteralPhase::One]),
                cube(&[LiteralPhase::One, LiteralPhase::Zero]),
            ],
        );
        let divisor = NodeCover::literal(A, LiteralPhase::One).unwrap();

        let division = divide(&dividend, &divisor, true).unwrap();

        assert_eq!(division.quotient.fanins(), &[B]);
        assert_eq!(
            division.quotient.cubes(),
            &[cube(&[LiteralPhase::One]), cube(&[LiteralPhase::Zero])]
        );
        assert_eq!(
            division.remainder.unwrap().cubes(),
            &[cube(&[LiteralPhase::Zero, LiteralPhase::One])]
        );
    }

    #[test]
    fn single_cube_division_uses_common_base() {
        let dividend = cover(
            &[A, B, C],
            vec![
                cube(&[LiteralPhase::One, LiteralPhase::Zero, LiteralPhase::One]),
                cube(&[LiteralPhase::One, LiteralPhase::One, LiteralPhase::One]),
            ],
        );
        let divisor = cover(&[C, A], vec![cube(&[LiteralPhase::One, LiteralPhase::One])]);

        let division = divide(&dividend, &divisor, true).unwrap();

        assert_eq!(division.quotient.fanins(), &[B]);
        assert_eq!(
            division.quotient.cubes(),
            &[cube(&[LiteralPhase::Zero]), cube(&[LiteralPhase::One])]
        );
        assert!(division.remainder.unwrap().is_zero());
    }

    #[test]
    fn multi_cube_division_keeps_candidates_seen_for_each_divisor_cube() {
        let dividend = cover(
            &[A, B, C],
            vec![
                cube(&[LiteralPhase::One, LiteralPhase::One, LiteralPhase::Zero]),
                cube(&[LiteralPhase::Zero, LiteralPhase::One, LiteralPhase::Zero]),
                cube(&[LiteralPhase::One, LiteralPhase::One, LiteralPhase::One]),
            ],
        );
        let divisor = cover(
            &[A],
            vec![cube(&[LiteralPhase::One]), cube(&[LiteralPhase::Zero])],
        );

        let division = divide(&dividend, &divisor, true).unwrap();

        assert_eq!(division.quotient.fanins(), &[B, C]);
        assert_eq!(
            division.quotient.cubes(),
            &[cube(&[LiteralPhase::One, LiteralPhase::Zero])]
        );
        assert_eq!(
            division.remainder.unwrap().cubes(),
            &[cube(&[
                LiteralPhase::One,
                LiteralPhase::One,
                LiteralPhase::One
            ])]
        );
    }

    #[test]
    fn trivial_filters_return_zero_quotient_and_original_remainder() {
        let dividend = cover(&[A], vec![cube(&[LiteralPhase::One])]);
        let larger_divisor = cover(
            &[A, B],
            vec![
                cube(&[LiteralPhase::One, LiteralPhase::DontCare]),
                cube(&[LiteralPhase::DontCare, LiteralPhase::One]),
            ],
        );
        let missing_support_divisor = cover(
            &[B, C],
            vec![
                cube(&[LiteralPhase::One, LiteralPhase::DontCare]),
                cube(&[LiteralPhase::DontCare, LiteralPhase::One]),
            ],
        );

        let too_few = divide(&dividend, &larger_divisor, true).unwrap();
        let missing_support = divide(&larger_divisor, &missing_support_divisor, true).unwrap();

        assert!(too_few.quotient.is_zero());
        assert_eq!(too_few.remainder.unwrap(), dividend);
        assert!(missing_support.quotient.is_zero());
        assert_eq!(missing_support.remainder.unwrap(), larger_divisor);
    }

    #[test]
    fn divide_by_zero_is_rejected() {
        let dividend = cover(&[A], vec![cube(&[LiteralPhase::One])]);

        assert_eq!(
            divide(&dividend, &NodeCover::constant(false), false),
            Err(DivideError::DivisionByZero)
        );
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_tokens_are_present() {
        let text = include_str!("divide.rs");

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
