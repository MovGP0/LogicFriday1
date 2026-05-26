//! Native Rust cube operations for Espresso-style set covers.
//!
//! These routines mirror the cube-level behavior of the original bitset
//! implementation without exposing per-file C ABI symbols. A `CubeStructure`
//! supplies the variable-to-part layout, and `Cube` stores the selected parts
//! as owned Rust values.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Variable {
    first_part: usize,
    last_part: usize,
}

impl Variable {
    pub const fn new(first_part: usize, last_part: usize) -> Self {
        Self {
            first_part,
            last_part,
        }
    }

    pub const fn first_part(self) -> usize {
        self.first_part
    }

    pub const fn last_part(self) -> usize {
        self.last_part
    }

    pub const fn part_count(self) -> usize {
        self.last_part - self.first_part + 1
    }

    pub const fn is_binary(self) -> bool {
        self.part_count() == 2
    }

    pub fn parts(self) -> impl Iterator<Item = usize> {
        self.first_part..=self.last_part
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeStructure {
    variables: Vec<Variable>,
    part_count: usize,
}

impl CubeStructure {
    pub fn new(variables: impl IntoIterator<Item = Variable>) -> SetcResult<Self> {
        let variables = variables.into_iter().collect::<Vec<_>>();
        let mut previous_last = None;
        for variable in &variables {
            if variable.first_part > variable.last_part {
                return Err(SetcError::EmptyVariable {
                    first_part: variable.first_part,
                    last_part: variable.last_part,
                });
            }

            if let Some(previous_last) = previous_last {
                if variable.first_part <= previous_last {
                    return Err(SetcError::OverlappingVariables);
                }
            }

            previous_last = Some(variable.last_part);
        }

        let part_count = variables
            .last()
            .map(|variable| variable.last_part + 1)
            .unwrap_or(0);

        Ok(Self {
            variables,
            part_count,
        })
    }

    pub fn variables(&self) -> &[Variable] {
        &self.variables
    }

    pub fn variable(&self, index: usize) -> Option<Variable> {
        self.variables.get(index).copied()
    }

    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    pub fn part_count(&self) -> usize {
        self.part_count
    }

    pub fn full_cube(&self) -> Cube {
        Cube::from_parts(0..self.part_count)
    }

    fn ensure_cube(&self, cube: &Cube) -> SetcResult<()> {
        for part in cube.parts() {
            if part >= self.part_count {
                return Err(SetcError::PartOutOfRange {
                    part,
                    part_count: self.part_count,
                });
            }
        }

        Ok(())
    }

    fn ensure_cubes<'a>(&self, cubes: impl IntoIterator<Item = &'a Cube>) -> SetcResult<()> {
        for cube in cubes {
            self.ensure_cube(cube)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    parts: BTreeSet<usize>,
}

impl Cube {
    pub fn empty() -> Self {
        Self {
            parts: BTreeSet::new(),
        }
    }

    pub fn from_parts(parts: impl IntoIterator<Item = usize>) -> Self {
        Self {
            parts: parts.into_iter().collect(),
        }
    }

    pub fn parts(&self) -> impl Iterator<Item = usize> + '_ {
        self.parts.iter().copied()
    }

    pub fn contains(&self, part: usize) -> bool {
        self.parts.contains(&part)
    }

    pub fn insert(&mut self, part: usize) -> bool {
        self.parts.insert(part)
    }

    pub fn len(&self) -> usize {
        self.parts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            parts: self.parts.union(&other.parts).copied().collect(),
        }
    }

    pub fn intersection(&self, other: &Self) -> Self {
        Self {
            parts: self.parts.intersection(&other.parts).copied().collect(),
        }
    }

    pub fn variable_parts(&self, variable: Variable) -> BTreeSet<usize> {
        variable
            .parts()
            .filter(|part| self.contains(*part))
            .collect()
    }

    pub fn contains_all_parts(&self, variable: Variable) -> bool {
        variable.parts().all(|part| self.contains(part))
    }

    pub fn intersects_variable(&self, other: &Self, variable: Variable) -> bool {
        variable
            .parts()
            .any(|part| self.contains(part) && other.contains(part))
    }

    fn has_missing_part_after_cofactor(&self, cofactor: &Self, variable: Variable) -> bool {
        variable
            .parts()
            .any(|part| !self.contains(part) && !cofactor.contains(part))
    }

    fn ordering_words(&self, structure: &CubeStructure) -> Vec<bool> {
        (0..structure.part_count())
            .rev()
            .map(|part| self.contains(part))
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SetcError {
    EmptyVariable { first_part: usize, last_part: usize },
    OverlappingVariables,
    PartOutOfRange { part: usize, part_count: usize },
}

impl fmt::Display for SetcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyVariable {
                first_part,
                last_part,
            } => write!(
                formatter,
                "variable part range {first_part}..={last_part} is empty"
            ),
            Self::OverlappingVariables => write!(formatter, "variable part ranges overlap"),
            Self::PartOutOfRange { part, part_count } => {
                write!(formatter, "cube part {part} is outside 0..{part_count}")
            }
        }
    }
}

impl Error for SetcError {}

pub type SetcResult<T> = Result<T, SetcError>;

pub fn full_row(cube: &Cube, cofactor: &Cube, structure: &CubeStructure) -> SetcResult<bool> {
    structure.ensure_cubes([cube, cofactor])?;

    Ok(structure.variables().iter().copied().all(|variable| {
        variable
            .parts()
            .all(|part| cube.contains(part) || cofactor.contains(part))
    }))
}

pub fn cube_distance_is_zero(
    left: &Cube,
    right: &Cube,
    structure: &CubeStructure,
) -> SetcResult<bool> {
    Ok(cube_distance(left, right, structure)? == 0)
}

pub fn cube_distance01(left: &Cube, right: &Cube, structure: &CubeStructure) -> SetcResult<usize> {
    structure.ensure_cubes([left, right])?;

    let mut distance = 0;
    for variable in structure.variables().iter().copied() {
        if !left.intersects_variable(right, variable) {
            distance += 1;
            if distance > 1 {
                return Ok(2);
            }
        }
    }

    Ok(distance)
}

pub fn cube_distance(left: &Cube, right: &Cube, structure: &CubeStructure) -> SetcResult<usize> {
    structure.ensure_cubes([left, right])?;

    Ok(structure
        .variables()
        .iter()
        .copied()
        .filter(|variable| !left.intersects_variable(right, *variable))
        .count())
}

pub fn force_lower(
    lower: &mut Cube,
    left: &Cube,
    right: &Cube,
    structure: &CubeStructure,
) -> SetcResult<()> {
    structure.ensure_cubes([lower, left, right])?;

    for variable in structure.variables().iter().copied() {
        if left.intersects_variable(right, variable) {
            continue;
        }

        for part in variable.parts().filter(|part| left.contains(*part)) {
            lower.insert(part);
        }
    }

    Ok(())
}

pub fn consensus(left: &Cube, right: &Cube, structure: &CubeStructure) -> SetcResult<Cube> {
    structure.ensure_cubes([left, right])?;

    let mut result = Cube::empty();
    for variable in structure.variables().iter().copied() {
        if left.intersects_variable(right, variable) {
            for part in variable
                .parts()
                .filter(|part| left.contains(*part) && right.contains(*part))
            {
                result.insert(part);
            }
        } else {
            for part in variable
                .parts()
                .filter(|part| left.contains(*part) || right.contains(*part))
            {
                result.insert(part);
            }
        }
    }

    Ok(result)
}

pub fn single_active_variable(cube: &Cube, structure: &CubeStructure) -> SetcResult<Option<usize>> {
    structure.ensure_cube(cube)?;

    let mut active = None;
    for (index, variable) in structure.variables().iter().copied().enumerate() {
        if cube.contains_all_parts(variable) {
            continue;
        }

        if active.is_some() {
            return Ok(None);
        }

        active = Some(index);
    }

    Ok(active)
}

pub fn has_common_active_variable(
    left: &Cube,
    right: &Cube,
    cofactor: &Cube,
    structure: &CubeStructure,
) -> SetcResult<bool> {
    structure.ensure_cubes([left, right, cofactor])?;

    Ok(structure.variables().iter().copied().any(|variable| {
        left.has_missing_part_after_cofactor(cofactor, variable)
            && right.has_missing_part_after_cofactor(cofactor, variable)
    }))
}

pub fn descend(left: Option<&Cube>, right: Option<&Cube>, structure: &CubeStructure) -> Ordering {
    compare_with_nulls(left, right, structure, SortDirection::Descending)
}

pub fn ascend(left: Option<&Cube>, right: Option<&Cube>, structure: &CubeStructure) -> Ordering {
    compare_with_nulls(left, right, structure, SortDirection::Ascending)
}

pub fn lex_order(left: &Cube, right: &Cube, structure: &CubeStructure) -> Ordering {
    compare_words(left, right, structure, SortDirection::Descending)
}

pub fn distance_one_order(
    left: &Cube,
    right: &Cube,
    mask: &Cube,
    structure: &CubeStructure,
) -> Ordering {
    let masked_left = left.union(mask);
    let masked_right = right.union(mask);

    compare_words(
        &masked_left,
        &masked_right,
        structure,
        SortDirection::Descending,
    )
}

fn compare_with_nulls(
    left: Option<&Cube>,
    right: Option<&Cube>,
    structure: &CubeStructure,
    direction: SortDirection,
) -> Ordering {
    match (left, right) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(left), Some(right)) => compare_by_size_then_words(left, right, structure, direction),
    }
}

fn compare_by_size_then_words(
    left: &Cube,
    right: &Cube,
    structure: &CubeStructure,
    direction: SortDirection,
) -> Ordering {
    match left.len().cmp(&right.len()) {
        Ordering::Greater => direction.large_first(),
        Ordering::Less => direction.small_first(),
        Ordering::Equal => compare_words(left, right, structure, direction),
    }
}

fn compare_words(
    left: &Cube,
    right: &Cube,
    structure: &CubeStructure,
    direction: SortDirection,
) -> Ordering {
    match left
        .ordering_words(structure)
        .cmp(&right.ordering_words(structure))
    {
        Ordering::Greater => direction.large_first(),
        Ordering::Less => direction.small_first(),
        Ordering::Equal => Ordering::Equal,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    const fn large_first(self) -> Ordering {
        match self {
            Self::Ascending => Ordering::Greater,
            Self::Descending => Ordering::Less,
        }
    }

    const fn small_first(self) -> Ordering {
        match self {
            Self::Ascending => Ordering::Less,
            Self::Descending => Ordering::Greater,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structure() -> CubeStructure {
        CubeStructure::new([
            Variable::new(0, 1),
            Variable::new(2, 3),
            Variable::new(4, 6),
        ])
        .unwrap()
    }

    fn cube(parts: &[usize]) -> Cube {
        Cube::from_parts(parts.iter().copied())
    }

    #[test]
    fn full_row_accepts_cube_and_cofactor_covering_every_part() {
        let structure = structure();

        assert!(full_row(&cube(&[0, 2, 4]), &cube(&[1, 3, 5, 6]), &structure).unwrap());
        assert!(!full_row(&cube(&[0, 2, 4]), &cube(&[1, 5, 6]), &structure).unwrap());
    }

    #[test]
    fn distance_counts_variables_with_empty_intersections() {
        let structure = structure();
        let left = cube(&[0, 2, 4, 5]);
        let right = cube(&[1, 3, 6]);

        assert_eq!(cube_distance(&left, &right, &structure).unwrap(), 3);
        assert_eq!(cube_distance01(&left, &right, &structure).unwrap(), 2);
        assert!(!cube_distance_is_zero(&left, &right, &structure).unwrap());

        let overlapping = cube(&[0, 2, 5]);
        assert_eq!(cube_distance(&left, &overlapping, &structure).unwrap(), 0);
        assert!(cube_distance_is_zero(&left, &overlapping, &structure).unwrap());
    }

    #[test]
    fn distance01_preserves_zero_and_one_before_saturating() {
        let structure = structure();

        assert_eq!(
            cube_distance01(&cube(&[0, 2, 4]), &cube(&[0, 2, 4]), &structure).unwrap(),
            0
        );
        assert_eq!(
            cube_distance01(&cube(&[0, 2, 4]), &cube(&[0, 3, 4]), &structure).unwrap(),
            1
        );
        assert_eq!(
            cube_distance01(&cube(&[0, 2, 4]), &cube(&[1, 3, 6]), &structure).unwrap(),
            2
        );
    }

    #[test]
    fn force_lower_adds_left_parts_for_disjoint_variables_only() {
        let structure = structure();
        let mut lower = cube(&[6]);

        force_lower(
            &mut lower,
            &cube(&[0, 2, 4, 5]),
            &cube(&[1, 2, 6]),
            &structure,
        )
        .unwrap();

        assert_eq!(lower, cube(&[0, 4, 5, 6]));
    }

    #[test]
    fn consensus_intersects_nonempty_variables_and_unions_empty_ones() {
        let structure = structure();
        let result = consensus(&cube(&[0, 2, 4, 5]), &cube(&[1, 2, 6]), &structure).unwrap();

        assert_eq!(result, cube(&[0, 1, 2, 4, 5, 6]));
    }

    #[test]
    fn single_active_variable_reports_exactly_one_nonfull_variable() {
        let structure = structure();

        assert_eq!(
            single_active_variable(&cube(&[0, 1, 2, 4, 5, 6]), &structure).unwrap(),
            Some(1)
        );
        assert_eq!(
            single_active_variable(&structure.full_cube(), &structure).unwrap(),
            None
        );
        assert_eq!(
            single_active_variable(&cube(&[0, 2, 4, 5, 6]), &structure).unwrap(),
            None
        );
    }

    #[test]
    fn common_active_variable_respects_cofactor_parts() {
        let structure = structure();
        let left = cube(&[0, 2, 4, 5]);
        let right = cube(&[1, 3, 4]);

        assert!(has_common_active_variable(&left, &right, &cube(&[]), &structure).unwrap());
        assert!(
            !has_common_active_variable(&left, &right, &cube(&[0, 1, 2, 3, 5, 6]), &structure)
                .unwrap()
        );
    }

    #[test]
    fn sorting_matches_size_then_high_part_order() {
        let structure = structure();
        let small = cube(&[0, 2]);
        let low = cube(&[0, 2, 4]);
        let high = cube(&[0, 2, 6]);

        assert_eq!(
            descend(Some(&low), Some(&small), &structure),
            Ordering::Less
        );
        assert_eq!(
            ascend(Some(&low), Some(&small), &structure),
            Ordering::Greater
        );
        assert_eq!(lex_order(&high, &low, &structure), Ordering::Less);
        assert_eq!(descend(None, Some(&small), &structure), Ordering::Greater);
    }

    #[test]
    fn distance_one_order_ignores_masked_parts() {
        let structure = structure();
        let left = cube(&[0, 2, 4]);
        let right = cube(&[0, 2, 5]);

        assert_eq!(lex_order(&right, &left, &structure), Ordering::Less);
        assert_eq!(
            distance_one_order(&left, &right, &cube(&[4, 5]), &structure),
            Ordering::Equal
        );
    }

    #[test]
    fn rejects_parts_outside_structure() {
        let structure = structure();

        assert_eq!(
            cube_distance(&cube(&[0, 7]), &cube(&[0]), &structure).unwrap_err(),
            SetcError::PartOutOfRange {
                part: 7,
                part_count: 7,
            }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present() {
        let source = include_str!("setc.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
    }
}
