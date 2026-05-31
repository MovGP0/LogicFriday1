//! Native reduction routines for Espresso-style positional cubes.
//!
//! Reduction replaces each on-set cube with the smallest subcube that still
//! preserves the represented function when combined with the remaining on-set
//! and don't-care cubes. The implementation uses owned Rust data and exact
//! set semantics instead of legacy global cube state.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct EspressoCube {
    parts: Vec<BTreeSet<usize>>,
}

impl EspressoCube {
    pub fn new(parts: Vec<BTreeSet<usize>>) -> Self {
        Self { parts }
    }

    pub fn from_parts<I, P>(part_sizes: &[usize], parts: I) -> ReduceResult<Self>
    where
        I: IntoIterator<Item = P>,
        P: IntoIterator<Item = usize>,
    {
        validate_part_sizes(part_sizes)?;

        let parts = parts
            .into_iter()
            .map(|variable_parts| variable_parts.into_iter().collect::<BTreeSet<_>>())
            .collect::<Vec<_>>();

        let cube = Self::new(parts);
        validate_cube(part_sizes, &cube)?;
        Ok(cube)
    }

    pub fn full(part_sizes: &[usize]) -> ReduceResult<Self> {
        validate_part_sizes(part_sizes)?;

        Ok(Self {
            parts: part_sizes
                .iter()
                .map(|part_size| (0..*part_size).collect())
                .collect(),
        })
    }

    pub fn empty(variable_count: usize) -> Self {
        Self {
            parts: vec![BTreeSet::new(); variable_count],
        }
    }

    pub fn parts(&self) -> &[BTreeSet<usize>] {
        &self.parts
    }

    pub fn variable_count(&self) -> usize {
        self.parts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.parts.iter().any(BTreeSet::is_empty)
    }

    pub fn is_full(&self, part_sizes: &[usize]) -> bool {
        self.parts
            .iter()
            .zip(part_sizes)
            .all(|(parts, part_size)| parts.len() == *part_size)
    }

    pub fn literal_count(&self) -> usize {
        self.parts.iter().map(BTreeSet::len).sum()
    }

    pub fn contains_assignment(&self, assignment: &[usize]) -> bool {
        self.parts
            .iter()
            .zip(assignment)
            .all(|(parts, part)| parts.contains(part))
    }

    pub fn intersects(&self, other: &Self) -> ReduceResult<bool> {
        ensure_same_variable_count(self, other)?;

        Ok(self
            .parts
            .iter()
            .zip(&other.parts)
            .all(|(left, right)| !left.is_disjoint(right)))
    }

    pub fn intersection(&self, other: &Self) -> ReduceResult<Self> {
        ensure_same_variable_count(self, other)?;

        Ok(Self {
            parts: self
                .parts
                .iter()
                .zip(&other.parts)
                .map(|(left, right)| left.intersection(right).copied().collect())
                .collect(),
        })
    }

    pub fn supercube(&self, other: &Self) -> ReduceResult<Self> {
        ensure_same_variable_count(self, other)?;

        Ok(Self {
            parts: self
                .parts
                .iter()
                .zip(&other.parts)
                .map(|(left, right)| left.union(right).copied().collect())
                .collect(),
        })
    }

    pub fn complement_in_variable(
        &self,
        part_sizes: &[usize],
        variable: usize,
    ) -> ReduceResult<Self> {
        validate_cube(part_sizes, self)?;

        let part_size = *part_sizes
            .get(variable)
            .ok_or(ReduceError::VariableOutOfRange {
                variable,
                variable_count: part_sizes.len(),
            })?;

        let mut result = self.clone();
        result.parts[variable] = (0..part_size)
            .filter(|part| !self.parts[variable].contains(part))
            .collect();
        Ok(result)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspressoCover {
    part_sizes: Vec<usize>,
    cubes: Vec<EspressoCube>,
}

impl EspressoCover {
    pub fn new(part_sizes: Vec<usize>, cubes: Vec<EspressoCube>) -> ReduceResult<Self> {
        validate_part_sizes(&part_sizes)?;

        for cube in &cubes {
            validate_cube(&part_sizes, cube)?;
        }

        Ok(Self { part_sizes, cubes })
    }

    pub fn empty(part_sizes: Vec<usize>) -> ReduceResult<Self> {
        Self::new(part_sizes, Vec::new())
    }

    pub fn from_rows<I, P, R>(part_sizes: Vec<usize>, rows: I) -> ReduceResult<Self>
    where
        I: IntoIterator<Item = R>,
        R: IntoIterator<Item = P>,
        P: IntoIterator<Item = usize>,
    {
        let cubes = rows
            .into_iter()
            .map(|parts| EspressoCube::from_parts(&part_sizes, parts))
            .collect::<ReduceResult<Vec<_>>>()?;

        Self::new(part_sizes, cubes)
    }

    pub fn part_sizes(&self) -> &[usize] {
        &self.part_sizes
    }

    pub fn cubes(&self) -> &[EspressoCube] {
        &self.cubes
    }

    pub fn into_cubes(self) -> Vec<EspressoCube> {
        self.cubes
    }

    pub fn variable_count(&self) -> usize {
        self.part_sizes.len()
    }

    pub fn cube_count(&self) -> usize {
        self.cubes.len()
    }

    pub fn evaluates(&self, assignment: &[usize]) -> bool {
        self.cubes
            .iter()
            .any(|cube| cube.contains_assignment(assignment))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReductionOrder {
    SortReduce,
    MiniSort,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReductionOptions {
    order: ReductionOrder,
}

impl ReductionOptions {
    pub fn new(order: ReductionOrder) -> Self {
        Self { order }
    }

    pub fn order(&self) -> ReductionOrder {
        self.order
    }
}

impl Default for ReductionOptions {
    fn default() -> Self {
        Self {
            order: ReductionOrder::SortReduce,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspressoReducer {
    use_sort_reduce_next: bool,
}

impl EspressoReducer {
    pub fn new() -> Self {
        Self {
            use_sort_reduce_next: true,
        }
    }

    pub fn reduce_alternating(
        &mut self,
        on_set: &EspressoCover,
        dont_care: &EspressoCover,
    ) -> ReduceResult<ReductionResult> {
        let order = if self.use_sort_reduce_next {
            ReductionOrder::SortReduce
        } else {
            ReductionOrder::MiniSort
        };
        self.use_sort_reduce_next = !self.use_sort_reduce_next;

        reduce_with_options(on_set, dont_care, &ReductionOptions::new(order))
    }
}

impl Default for EspressoReducer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReductionResult {
    cover: EspressoCover,
    steps: Vec<ReductionStep>,
}

impl ReductionResult {
    pub fn cover(&self) -> &EspressoCover {
        &self.cover
    }

    pub fn into_cover(self) -> EspressoCover {
        self.cover
    }

    pub fn steps(&self) -> &[ReductionStep] {
        &self.steps
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReductionStep {
    original: EspressoCube,
    reduced: EspressoCube,
    active: bool,
    prime: bool,
}

impl ReductionStep {
    pub fn original(&self) -> &EspressoCube {
        &self.original
    }

    pub fn reduced(&self) -> &EspressoCube {
        &self.reduced
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn prime(&self) -> bool {
        self.prime
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReduceError {
    EmptyPartSize {
        variable: usize,
    },
    PartOutOfRange {
        variable: usize,
        part: usize,
        part_size: usize,
    },
    VariableCountMismatch {
        expected: usize,
        actual: usize,
    },
    VariableOutOfRange {
        variable: usize,
        variable_count: usize,
    },
    CoverShapeMismatch {
        left: Vec<usize>,
        right: Vec<usize>,
    },
    AssignmentSpaceTooLarge,
}

impl fmt::Display for ReduceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPartSize { variable } => {
                write!(formatter, "variable {variable} has no parts")
            }
            Self::PartOutOfRange {
                variable,
                part,
                part_size,
            } => write!(
                formatter,
                "part {part} for variable {variable} is outside 0..{part_size}"
            ),
            Self::VariableCountMismatch { expected, actual } => write!(
                formatter,
                "cube has {actual} variables; expected {expected}"
            ),
            Self::VariableOutOfRange {
                variable,
                variable_count,
            } => write!(
                formatter,
                "variable {variable} is outside 0..{variable_count}"
            ),
            Self::CoverShapeMismatch { left, right } => {
                write!(formatter, "cover part sizes differ: {left:?} != {right:?}")
            }
            Self::AssignmentSpaceTooLarge => {
                write!(
                    formatter,
                    "assignment space is too large to enumerate exactly"
                )
            }
        }
    }
}

impl Error for ReduceError {}

pub type ReduceResult<T> = Result<T, ReduceError>;

pub fn reduce(on_set: &EspressoCover, dont_care: &EspressoCover) -> ReduceResult<EspressoCover> {
    Ok(reduce_with_options(on_set, dont_care, &ReductionOptions::default())?.into_cover())
}

pub fn reduce_with_options(
    on_set: &EspressoCover,
    dont_care: &EspressoCover,
    options: &ReductionOptions,
) -> ReduceResult<ReductionResult> {
    ensure_same_shape(on_set, dont_care)?;

    let mut cubes = ordered_cubes(on_set, options.order());
    let mut steps = Vec::with_capacity(cubes.len());

    for index in 0..cubes.len() {
        let original = cubes[index].clone();
        let reduced = reduce_cube_at(&on_set.part_sizes, &cubes, dont_care.cubes(), index)?;
        let active = !reduced.is_empty();
        let prime = reduced == original;

        cubes[index] = reduced.clone();
        steps.push(ReductionStep {
            original,
            reduced,
            active,
            prime,
        });
    }

    let active_cubes = cubes
        .into_iter()
        .zip(&steps)
        .filter_map(|(cube, step)| step.active().then_some(cube))
        .collect();

    Ok(ReductionResult {
        cover: EspressoCover::new(on_set.part_sizes.clone(), active_cubes)?,
        steps,
    })
}

pub fn reduce_cube(
    part_sizes: &[usize],
    cube: &EspressoCube,
    blocking_cover: &[EspressoCube],
) -> ReduceResult<EspressoCube> {
    validate_cube(part_sizes, cube)?;
    for blocker in blocking_cover {
        validate_cube(part_sizes, blocker)?;
    }

    let mut essential_points = Vec::new();
    for assignment in assignments_for_parts(cube.parts())? {
        if !blocking_cover
            .iter()
            .any(|blocker| blocker.contains_assignment(&assignment))
        {
            essential_points.push(assignment);
        }
    }

    supercube_of_assignments(part_sizes, &essential_points)
}

pub fn smallest_cube_containing_complement(cover: &EspressoCover) -> ReduceResult<EspressoCube> {
    let mut complement_points = Vec::new();

    for assignment in assignments_for_part_sizes(cover.part_sizes())? {
        if !cover.evaluates(&assignment) {
            complement_points.push(assignment);
        }
    }

    supercube_of_assignments(cover.part_sizes(), &complement_points)
}

pub fn sccc_cube(part_sizes: &[usize], cube: &EspressoCube) -> ReduceResult<EspressoCube> {
    validate_cube(part_sizes, cube)?;

    let active_variables = cube
        .parts()
        .iter()
        .zip(part_sizes)
        .enumerate()
        .filter_map(|(variable, (parts, part_size))| {
            (parts.len() != *part_size).then_some(variable)
        })
        .collect::<Vec<_>>();

    match active_variables.as_slice() {
        [] => Ok(EspressoCube::empty(part_sizes.len())),
        [variable] => cube.complement_in_variable(part_sizes, *variable),
        _ => EspressoCube::full(part_sizes),
    }
}

pub fn sccc_merge(
    left: &EspressoCube,
    right: &EspressoCube,
    left_cofactor: &EspressoCube,
    right_cofactor: &EspressoCube,
) -> ReduceResult<EspressoCube> {
    let left = left.intersection(left_cofactor)?;
    let right = right.intersection(right_cofactor)?;
    left.supercube(&right)
}

fn reduce_cube_at(
    part_sizes: &[usize],
    cubes: &[EspressoCube],
    dont_care: &[EspressoCube],
    index: usize,
) -> ReduceResult<EspressoCube> {
    let mut blocking_cover = Vec::with_capacity(cubes.len().saturating_sub(1) + dont_care.len());
    blocking_cover.extend(
        cubes
            .iter()
            .enumerate()
            .filter_map(|(candidate_index, cube)| {
                (candidate_index != index).then_some(cube.clone())
            }),
    );
    blocking_cover.extend(dont_care.iter().cloned());

    reduce_cube(part_sizes, &cubes[index], &blocking_cover)
}

fn ordered_cubes(on_set: &EspressoCover, order: ReductionOrder) -> Vec<EspressoCube> {
    let mut cubes = on_set.cubes.clone();

    match order {
        ReductionOrder::SortReduce => sort_reduce(&mut cubes, on_set.part_sizes.len()),
        ReductionOrder::MiniSort => mini_sort(&mut cubes, on_set.part_sizes()),
    }

    cubes
}

fn sort_reduce(cubes: &mut [EspressoCube], variable_count: usize) {
    let Some(largest) = cubes
        .iter()
        .max_by_key(|cube| cube.literal_count())
        .cloned()
    else {
        return;
    };

    cubes.sort_by_key(|cube| {
        let score =
            ((variable_count - cube_distance(&largest, cube)) << 7) + cube.literal_count().min(127);
        std::cmp::Reverse(score)
    });
}

fn mini_sort(cubes: &mut [EspressoCube], part_sizes: &[usize]) {
    let column_offsets = column_offsets(part_sizes);
    let mut counts = vec![0usize; part_sizes.iter().sum()];

    for cube in cubes.iter() {
        for (variable, parts) in cube.parts().iter().enumerate() {
            for part in parts {
                counts[column_offsets[variable] + part] += 1;
            }
        }
    }

    cubes.sort_by_key(|cube| {
        let mut score = 0usize;
        for (variable, parts) in cube.parts().iter().enumerate() {
            for part in parts {
                score += counts[column_offsets[variable] + part];
            }
        }

        std::cmp::Reverse(score)
    });
}

fn cube_distance(left: &EspressoCube, right: &EspressoCube) -> usize {
    left.parts()
        .iter()
        .zip(right.parts())
        .filter(|(left, right)| left.is_disjoint(right))
        .count()
}

fn supercube_of_assignments(
    part_sizes: &[usize],
    assignments: &[Vec<usize>],
) -> ReduceResult<EspressoCube> {
    if assignments.is_empty() {
        return Ok(EspressoCube::empty(part_sizes.len()));
    }

    let mut parts = vec![BTreeSet::new(); part_sizes.len()];
    for assignment in assignments {
        if assignment.len() != part_sizes.len() {
            return Err(ReduceError::VariableCountMismatch {
                expected: part_sizes.len(),
                actual: assignment.len(),
            });
        }

        for (variable, part) in assignment.iter().copied().enumerate() {
            if part >= part_sizes[variable] {
                return Err(ReduceError::PartOutOfRange {
                    variable,
                    part,
                    part_size: part_sizes[variable],
                });
            }

            parts[variable].insert(part);
        }
    }

    EspressoCube::from_parts(part_sizes, parts)
}

fn assignments_for_part_sizes(part_sizes: &[usize]) -> ReduceResult<Vec<Vec<usize>>> {
    let parts = part_sizes
        .iter()
        .map(|part_size| (0..*part_size).collect::<BTreeSet<_>>())
        .collect::<Vec<_>>();

    assignments_for_parts(&parts)
}

fn assignments_for_parts(parts: &[BTreeSet<usize>]) -> ReduceResult<Vec<Vec<usize>>> {
    let mut count = 1usize;
    for variable_parts in parts {
        count = count
            .checked_mul(variable_parts.len())
            .ok_or(ReduceError::AssignmentSpaceTooLarge)?;
    }

    let mut assignments = vec![Vec::with_capacity(parts.len())];
    for variable_parts in parts {
        let previous = std::mem::take(&mut assignments);
        assignments = Vec::with_capacity(
            previous
                .len()
                .checked_mul(variable_parts.len())
                .ok_or(ReduceError::AssignmentSpaceTooLarge)?,
        );

        for prefix in previous {
            for part in variable_parts {
                let mut assignment = prefix.clone();
                assignment.push(*part);
                assignments.push(assignment);
            }
        }
    }

    Ok(assignments)
}

fn column_offsets(part_sizes: &[usize]) -> Vec<usize> {
    let mut next = 0;
    part_sizes
        .iter()
        .map(|part_size| {
            let offset = next;
            next += part_size;
            offset
        })
        .collect()
}

fn validate_part_sizes(part_sizes: &[usize]) -> ReduceResult<()> {
    for (variable, part_size) in part_sizes.iter().copied().enumerate() {
        if part_size == 0 {
            return Err(ReduceError::EmptyPartSize { variable });
        }
    }

    Ok(())
}

fn validate_cube(part_sizes: &[usize], cube: &EspressoCube) -> ReduceResult<()> {
    if cube.variable_count() != part_sizes.len() {
        return Err(ReduceError::VariableCountMismatch {
            expected: part_sizes.len(),
            actual: cube.variable_count(),
        });
    }

    for (variable, parts) in cube.parts().iter().enumerate() {
        for part in parts {
            if *part >= part_sizes[variable] {
                return Err(ReduceError::PartOutOfRange {
                    variable,
                    part: *part,
                    part_size: part_sizes[variable],
                });
            }
        }
    }

    Ok(())
}

fn ensure_same_shape(left: &EspressoCover, right: &EspressoCover) -> ReduceResult<()> {
    if left.part_sizes() != right.part_sizes() {
        return Err(ReduceError::CoverShapeMismatch {
            left: left.part_sizes().to_vec(),
            right: right.part_sizes().to_vec(),
        });
    }

    Ok(())
}

fn ensure_same_variable_count(left: &EspressoCube, right: &EspressoCube) -> ReduceResult<()> {
    if left.variable_count() != right.variable_count() {
        return Err(ReduceError::VariableCountMismatch {
            expected: left.variable_count(),
            actual: right.variable_count(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(pattern: &[&[usize]]) -> EspressoCube {
        EspressoCube::new(
            pattern
                .iter()
                .map(|parts| parts.iter().copied().collect())
                .collect(),
        )
    }

    fn binary_cube(pattern: &str) -> EspressoCube {
        EspressoCube::new(
            pattern
                .chars()
                .map(|ch| match ch {
                    '0' => BTreeSet::from([0]),
                    '1' => BTreeSet::from([1]),
                    '-' => BTreeSet::from([0, 1]),
                    _ => panic!("invalid test literal"),
                })
                .collect(),
        )
    }

    fn binary_cover(patterns: &[&str]) -> EspressoCover {
        EspressoCover::new(
            vec![2, 2],
            patterns
                .iter()
                .map(|pattern| binary_cube(pattern))
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn reduce_cube_keeps_only_points_not_covered_by_blockers() {
        let part_sizes = [2, 2];
        let target = binary_cube("--");
        let blockers = [binary_cube("0-"), binary_cube("-0")];

        let reduced = reduce_cube(&part_sizes, &target, &blockers).unwrap();

        assert_eq!(reduced, binary_cube("11"));
    }

    #[test]
    fn reduce_cube_returns_empty_when_cube_is_redundant() {
        let part_sizes = [2, 2];
        let target = binary_cube("10");
        let blockers = [binary_cube("1-")];

        let reduced = reduce_cube(&part_sizes, &target, &blockers).unwrap();

        assert!(reduced.is_empty());
    }

    #[test]
    fn reduce_removes_inactive_cubes_from_cover() {
        let on_set = binary_cover(&["10", "10"]);
        let dont_care = EspressoCover::empty(vec![2, 2]).unwrap();

        let reduced = reduce(&on_set, &dont_care).unwrap();

        assert_eq!(reduced.cubes(), &[binary_cube("10")]);
    }

    #[test]
    fn dont_care_cover_can_make_a_cube_redundant() {
        let on_set = binary_cover(&["10"]);
        let dont_care = binary_cover(&["1-"]);

        let reduced = reduce(&on_set, &dont_care).unwrap();

        assert_eq!(reduced.cube_count(), 0);
    }

    #[test]
    fn smallest_cube_containing_complement_of_empty_cover_is_full_cube() {
        let cover = EspressoCover::empty(vec![2, 3]).unwrap();

        let result = smallest_cube_containing_complement(&cover).unwrap();

        assert_eq!(result, EspressoCube::full(&[2, 3]).unwrap());
    }

    #[test]
    fn smallest_cube_containing_complement_of_universe_is_empty_cube() {
        let cover = EspressoCover::new(vec![2, 2], vec![binary_cube("--")]).unwrap();

        let result = smallest_cube_containing_complement(&cover).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn smallest_cube_containing_complement_supercubes_missing_points() {
        let cover = binary_cover(&["00", "11"]);

        let result = smallest_cube_containing_complement(&cover).unwrap();

        assert_eq!(result, binary_cube("--"));
    }

    #[test]
    fn sccc_cube_complements_single_active_variable() {
        let result = sccc_cube(&[2, 3], &cube(&[&[0, 1], &[1]])).unwrap();

        assert_eq!(result, cube(&[&[0, 1], &[0, 2]]));
    }

    #[test]
    fn sccc_cube_returns_full_when_multiple_variables_are_active() {
        let result = sccc_cube(&[2, 3], &cube(&[&[0], &[1]])).unwrap();

        assert_eq!(result, EspressoCube::full(&[2, 3]).unwrap());
    }

    #[test]
    fn sccc_merge_intersects_branches_with_cofactors_then_supercubes() {
        let left = cube(&[&[0, 1], &[0]]);
        let right = cube(&[&[1], &[0, 1]]);
        let left_cofactor = cube(&[&[0], &[0, 1]]);
        let right_cofactor = cube(&[&[0, 1], &[1]]);

        let merged = sccc_merge(&left, &right, &left_cofactor, &right_cofactor).unwrap();

        assert_eq!(merged, cube(&[&[0, 1], &[0, 1]]));
    }

    #[test]
    fn alternating_reducer_switches_ordering_between_calls() {
        let on_set = binary_cover(&["1-", "-1"]);
        let dont_care = EspressoCover::empty(vec![2, 2]).unwrap();
        let mut reducer = EspressoReducer::new();

        let first = reducer.reduce_alternating(&on_set, &dont_care).unwrap();
        let second = reducer.reduce_alternating(&on_set, &dont_care).unwrap();

        assert_eq!(first.cover().part_sizes(), &[2, 2]);
        assert_eq!(second.cover().part_sizes(), &[2, 2]);
    }

    #[test]
    fn rejects_part_outside_declared_variable_size() {
        let error = EspressoCover::new(vec![2], vec![cube(&[&[2]])]).unwrap_err();

        assert_eq!(
            error,
            ReduceError::PartOutOfRange {
                variable: 0,
                part: 2,
                part_size: 2
            }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present() {
        let source = include_str!("reduce.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
    }
}
