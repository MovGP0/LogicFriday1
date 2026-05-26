//! Quick divisor extraction for SIS algebraic sum-of-products covers.
//!
//! The original routine repeatedly divides a temporary copy of a node by the
//! literal with the highest occurrence count. Extraction stops when every
//! literal appears at most once. A divisor is returned only if at least one
//! division was performed.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VariableId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LiteralPhase {
    Negative,
    Positive,
}

impl LiteralPhase {
    pub const fn complement(self) -> Self {
        match self {
            Self::Negative => Self::Positive,
            Self::Positive => Self::Negative,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Literal {
    pub variable: VariableId,
    pub phase: LiteralPhase,
}

impl Literal {
    pub const fn new(variable: VariableId, phase: LiteralPhase) -> Self {
        Self { variable, phase }
    }

    pub const fn positive(variable: VariableId) -> Self {
        Self::new(variable, LiteralPhase::Positive)
    }

    pub const fn negative(variable: VariableId) -> Self {
        Self::new(variable, LiteralPhase::Negative)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct QuickDivisorCube {
    literals: Vec<Literal>,
}

impl QuickDivisorCube {
    pub fn new(literals: impl Into<Vec<Literal>>) -> QuickDivisorResult<Self> {
        let mut literals = literals.into();
        literals.sort_unstable();
        literals.dedup();
        validate_literals(&literals)?;

        Ok(Self { literals })
    }

    pub fn one() -> Self {
        Self {
            literals: Vec::new(),
        }
    }

    pub fn literals(&self) -> &[Literal] {
        &self.literals
    }

    pub fn contains(&self, literal: Literal) -> bool {
        self.literals.binary_search(&literal).is_ok()
    }

    pub fn quotient_by_literal(&self, literal: Literal) -> Option<Self> {
        self.contains(literal).then(|| Self {
            literals: self
                .literals
                .iter()
                .copied()
                .filter(|candidate| *candidate != literal)
                .collect(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuickDivisorCover {
    cubes: Vec<QuickDivisorCube>,
}

impl QuickDivisorCover {
    pub fn new(cubes: impl Into<Vec<QuickDivisorCube>>) -> Self {
        let mut cubes = cubes.into();
        cubes.sort_unstable();
        cubes.dedup();

        Self { cubes }
    }

    pub fn zero() -> Self {
        Self { cubes: Vec::new() }
    }

    pub fn one() -> Self {
        Self {
            cubes: vec![QuickDivisorCube::one()],
        }
    }

    pub fn literal(literal: Literal) -> Self {
        Self {
            cubes: vec![QuickDivisorCube {
                literals: vec![literal],
            }],
        }
    }

    pub fn cubes(&self) -> &[QuickDivisorCube] {
        &self.cubes
    }

    pub fn is_zero(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn quotient_by_literal(&self, literal: Literal) -> Self {
        Self::new(
            self.cubes
                .iter()
                .filter_map(|cube| cube.quotient_by_literal(literal))
                .collect::<Vec<_>>(),
        )
    }

    pub fn literal_counts(&self) -> BTreeMap<Literal, usize> {
        let mut counts = BTreeMap::new();

        for cube in &self.cubes {
            for literal in cube.literals() {
                *counts.entry(*literal).or_insert(0) += 1;
            }
        }

        counts
    }

    pub fn evaluates(&self, assignment: &BTreeMap<VariableId, bool>) -> QuickDivisorResult<bool> {
        for cube in &self.cubes {
            let mut cube_value = true;

            for literal in cube.literals() {
                let Some(value) = assignment.get(&literal.variable) else {
                    return Err(QuickDivisorError::MissingAssignment {
                        variable: literal.variable,
                    });
                };

                let literal_value = match literal.phase {
                    LiteralPhase::Negative => !*value,
                    LiteralPhase::Positive => *value,
                };

                cube_value &= literal_value;
            }

            if cube_value {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuickDivisor {
    pub divisor: QuickDivisorCover,
    pub divided_literals: Vec<Literal>,
}

impl QuickDivisor {
    pub fn division_count(&self) -> usize {
        self.divided_literals.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuickDivisorError {
    ConflictingLiteral { variable: VariableId },
    MissingAssignment { variable: VariableId },
}

impl fmt::Display for QuickDivisorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingLiteral { variable } => {
                write!(f, "cube contains both phases of variable {}", variable.0)
            }
            Self::MissingAssignment { variable } => {
                write!(f, "missing assignment for variable {}", variable.0)
            }
        }
    }
}

impl Error for QuickDivisorError {}

pub type QuickDivisorResult<T> = Result<T, QuickDivisorError>;

pub fn find_quick_divisor(node: &QuickDivisorCover) -> Option<QuickDivisor> {
    let mut divisor = node.clone();
    let mut divided_literals = Vec::new();

    loop {
        let Some((literal, count)) = select_best_literal(&divisor) else {
            break;
        };

        if count <= 1 {
            break;
        }

        divisor = divisor.quotient_by_literal(literal);
        divided_literals.push(literal);
    }

    (!divided_literals.is_empty()).then_some(QuickDivisor {
        divisor,
        divided_literals,
    })
}

pub fn select_best_literal(cover: &QuickDivisorCover) -> Option<(Literal, usize)> {
    let counts = cover.literal_counts();
    let max_variable = counts.keys().map(|literal| literal.variable.0).max()?;
    let mut best = None;

    for variable_index in (0..=max_variable).rev() {
        for phase in [LiteralPhase::Negative, LiteralPhase::Positive] {
            let literal = Literal::new(VariableId(variable_index), phase);
            let count = *counts.get(&literal).unwrap_or(&0);

            if best
                .as_ref()
                .is_none_or(|(_best_literal, best_count)| count > *best_count)
            {
                best = Some((literal, count));
            }
        }
    }

    best
}

fn validate_literals(literals: &[Literal]) -> QuickDivisorResult<()> {
    let mut phases_by_variable = BTreeMap::new();

    for literal in literals {
        if let Some(previous) = phases_by_variable.insert(literal.variable, literal.phase) {
            if previous != literal.phase {
                return Err(QuickDivisorError::ConflictingLiteral {
                    variable: literal.variable,
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: VariableId = VariableId(0);
    const B: VariableId = VariableId(1);
    const C: VariableId = VariableId(2);
    const D: VariableId = VariableId(3);

    fn pos(variable: VariableId) -> Literal {
        Literal::positive(variable)
    }

    fn neg(variable: VariableId) -> Literal {
        Literal::negative(variable)
    }

    fn cube(literals: &[Literal]) -> QuickDivisorCube {
        QuickDivisorCube::new(literals.to_vec()).unwrap()
    }

    fn cover(cubes: Vec<QuickDivisorCube>) -> QuickDivisorCover {
        QuickDivisorCover::new(cubes)
    }

    #[test]
    fn repeated_best_literals_are_divided_until_no_literal_repeats() {
        let node = cover(vec![
            cube(&[pos(A), pos(B), pos(C)]),
            cube(&[pos(A), neg(B), pos(C)]),
            cube(&[pos(A), pos(D)]),
        ]);

        let quick = find_quick_divisor(&node).unwrap();

        assert_eq!(quick.divided_literals, vec![pos(A), pos(C)]);
        assert_eq!(quick.divisor, cover(vec![cube(&[pos(B)]), cube(&[neg(B)])]));
    }

    #[test]
    fn no_divisor_is_returned_when_every_literal_is_unique() {
        let node = cover(vec![cube(&[pos(A), pos(B)]), cube(&[pos(C), neg(D)])]);

        assert_eq!(find_quick_divisor(&node), None);
    }

    #[test]
    fn tie_breaking_matches_descending_legacy_literal_index_order() {
        let node = cover(vec![
            cube(&[pos(A), pos(B), neg(C)]),
            cube(&[neg(B), pos(C)]),
        ]);

        assert_eq!(select_best_literal(&node), Some((neg(C), 1)));
    }

    #[test]
    fn quotient_by_literal_keeps_only_cubes_containing_that_literal() {
        let node = cover(vec![
            cube(&[pos(A), pos(B)]),
            cube(&[pos(A), pos(C)]),
            cube(&[pos(D)]),
        ]);

        assert_eq!(
            node.quotient_by_literal(pos(A)),
            cover(vec![cube(&[pos(B)]), cube(&[pos(C)])])
        );
    }

    #[test]
    fn quotient_product_assignments_are_subset_of_original_function() {
        let node = cover(vec![
            cube(&[pos(A), pos(B), pos(C)]),
            cube(&[pos(A), neg(B), pos(C)]),
            cube(&[pos(A), pos(D)]),
        ]);
        let quick = find_quick_divisor(&node).unwrap();
        let assignments = [
            BTreeMap::from([(A, true), (B, true), (C, true), (D, false)]),
            BTreeMap::from([(A, true), (B, false), (C, true), (D, false)]),
            BTreeMap::from([(A, true), (B, false), (C, false), (D, true)]),
        ];

        for assignment in assignments {
            let quotient_product_is_true = quick.divided_literals.iter().all(|literal| {
                let value = assignment[&literal.variable];

                match literal.phase {
                    LiteralPhase::Negative => !value,
                    LiteralPhase::Positive => value,
                }
            }) && quick.divisor.evaluates(&assignment).unwrap();

            assert!(!quotient_product_is_true || node.evaluates(&assignment).unwrap());
        }
    }

    #[test]
    fn conflicting_cube_literals_are_rejected() {
        assert_eq!(
            QuickDivisorCube::new(vec![pos(A), neg(A)]),
            Err(QuickDivisorError::ConflictingLiteral { variable: A })
        );
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let source = include_str!("qdivisor.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("Logic", "Friday", "1-", "8j8")));
    }
}
