//! Native Rust sharp-product operations for Espresso-style covers.
//!
//! The routines in this module keep the original multi-valued cube semantics:
//! each variable owns a range of part positions, a cube covers assignments by
//! selecting one or more parts per variable, and containment is set superset
//! containment over those part positions.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SharpVariable {
    first_part: usize,
    last_part: usize,
}

impl SharpVariable {
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

    fn parts(self) -> impl Iterator<Item = usize> {
        self.first_part..=self.last_part
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SharpStructure {
    set_size: usize,
    variables: Vec<SharpVariable>,
}

impl SharpStructure {
    pub fn new(
        set_size: usize,
        variables: impl IntoIterator<Item = SharpVariable>,
    ) -> SharpResult<Self> {
        let variables = variables.into_iter().collect::<Vec<_>>();
        let mut covered_parts = BTreeSet::new();

        for variable in &variables {
            if variable.first_part() > variable.last_part() || variable.last_part() >= set_size {
                return Err(SharpError::InvalidVariable {
                    first_part: variable.first_part(),
                    last_part: variable.last_part(),
                    set_size,
                });
            }

            for part in variable.parts() {
                if !covered_parts.insert(part) {
                    return Err(SharpError::OverlappingVariablePart { part });
                }
            }
        }

        Ok(Self {
            set_size,
            variables,
        })
    }

    pub fn set_size(&self) -> usize {
        self.set_size
    }

    pub fn variables(&self) -> &[SharpVariable] {
        &self.variables
    }

    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SharpCube {
    set_size: usize,
    parts: BTreeSet<usize>,
}

impl SharpCube {
    pub fn empty(set_size: usize) -> Self {
        Self {
            set_size,
            parts: BTreeSet::new(),
        }
    }

    pub fn from_parts(
        set_size: usize,
        parts: impl IntoIterator<Item = usize>,
    ) -> SharpResult<Self> {
        let mut cube = Self::empty(set_size);
        for part in parts {
            cube.insert(part)?;
        }

        Ok(cube)
    }

    pub fn set_size(&self) -> usize {
        self.set_size
    }

    pub fn parts(&self) -> &BTreeSet<usize> {
        &self.parts
    }

    pub fn contains(&self, part: usize) -> bool {
        self.parts.contains(&part)
    }

    pub fn insert(&mut self, part: usize) -> SharpResult<bool> {
        if part >= self.set_size {
            return Err(SharpError::PartOutOfRange {
                part,
                set_size: self.set_size,
            });
        }

        Ok(self.parts.insert(part))
    }

    fn contains_all_parts_of(&self, other: &Self) -> bool {
        other.parts.is_subset(&self.parts)
    }

    fn intersects_variable_with(&self, other: &Self, variable: SharpVariable) -> bool {
        variable
            .parts()
            .any(|part| self.contains(part) && other.contains(part))
    }

    fn has_part_in_variable(&self, variable: SharpVariable) -> bool {
        variable.parts().any(|part| self.contains(part))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SharpCover {
    set_size: usize,
    cubes: Vec<SharpCube>,
}

impl SharpCover {
    pub fn new(set_size: usize, cubes: impl IntoIterator<Item = SharpCube>) -> SharpResult<Self> {
        let cubes = cubes.into_iter().collect::<Vec<_>>();
        for cube in &cubes {
            if cube.set_size() != set_size {
                return Err(SharpError::SetSizeMismatch {
                    left: set_size,
                    right: cube.set_size(),
                });
            }
        }

        Ok(Self { set_size, cubes })
    }

    pub fn empty(set_size: usize) -> Self {
        Self {
            set_size,
            cubes: Vec::new(),
        }
    }

    pub fn singleton(cube: SharpCube) -> Self {
        Self {
            set_size: cube.set_size(),
            cubes: vec![cube],
        }
    }

    pub fn set_size(&self) -> usize {
        self.set_size
    }

    pub fn cubes(&self) -> &[SharpCube] {
        &self.cubes
    }

    pub fn len(&self) -> usize {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn push(&mut self, cube: SharpCube) -> SharpResult<()> {
        if cube.set_size() != self.set_size {
            return Err(SharpError::SetSizeMismatch {
                left: self.set_size,
                right: cube.set_size(),
            });
        }

        self.cubes.push(cube);
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SharpError {
    InvalidVariable {
        first_part: usize,
        last_part: usize,
        set_size: usize,
    },
    OverlappingVariablePart {
        part: usize,
    },
    PartOutOfRange {
        part: usize,
        set_size: usize,
    },
    SetSizeMismatch {
        left: usize,
        right: usize,
    },
}

impl fmt::Display for SharpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVariable {
                first_part,
                last_part,
                set_size,
            } => write!(
                formatter,
                "variable part range {first_part}..={last_part} is invalid for set size {set_size}"
            ),
            Self::OverlappingVariablePart { part } => {
                write!(formatter, "part {part} appears in more than one variable")
            }
            Self::PartOutOfRange { part, set_size } => {
                write!(formatter, "part {part} is outside set size {set_size}")
            }
            Self::SetSizeMismatch { left, right } => {
                write!(
                    formatter,
                    "cube or cover set sizes differ: {left} != {right}"
                )
            }
        }
    }
}

impl Error for SharpError {}

pub type SharpResult<T> = Result<T, SharpError>;

pub fn cv_sharp(
    left: &SharpCover,
    right: &SharpCover,
    structure: &SharpStructure,
) -> SharpResult<SharpCover> {
    ensure_cover_shape(left, structure)?;
    ensure_cover_shape(right, structure)?;

    let mut result = SharpCover::empty(structure.set_size());
    for cube in left.cubes() {
        result = union_covers(result, cb_sharp(cube, right, structure)?)?;
    }

    Ok(result)
}

pub fn cb_sharp(
    cube: &SharpCube,
    cover: &SharpCover,
    structure: &SharpStructure,
) -> SharpResult<SharpCover> {
    ensure_cube_shape(cube, structure)?;
    ensure_cover_shape(cover, structure)?;

    if cover.is_empty() {
        return Ok(SharpCover::singleton(cube.clone()));
    }

    cb_recur_sharp(cube, cover, 0, cover.len() - 1, structure)
}

pub fn sharp(
    left: &SharpCube,
    right: &SharpCube,
    structure: &SharpStructure,
) -> SharpResult<SharpCover> {
    ensure_cube_shape(left, structure)?;
    ensure_cube_shape(right, structure)?;

    if !cubes_intersect(left, right, structure) {
        return Ok(SharpCover::singleton(left.clone()));
    }

    let diff = cube_difference(left, right);
    let mut result = SharpCover::empty(structure.set_size());
    for variable in structure.variables() {
        let variable_diff = cube_intersection_with_variable(&diff, *variable);
        if variable_diff.parts().is_empty() {
            continue;
        }

        let outside_variable = cube_difference_with_variable(left, *variable);
        result.push(cube_union(&variable_diff, &outside_variable))?;
    }

    Ok(result)
}

pub fn make_disjoint(cover: &SharpCover, structure: &SharpStructure) -> SharpResult<SharpCover> {
    ensure_cover_shape(cover, structure)?;

    let mut result = SharpCover::empty(structure.set_size());
    for cube in cover.cubes() {
        let new_cover = cb_dsharp(cube, &result, structure)?;
        result = append_covers(result, new_cover)?;
    }

    Ok(result)
}

pub fn cv_dsharp(
    left: &SharpCover,
    right: &SharpCover,
    structure: &SharpStructure,
) -> SharpResult<SharpCover> {
    ensure_cover_shape(left, structure)?;
    ensure_cover_shape(right, structure)?;

    let mut result = SharpCover::empty(structure.set_size());
    for cube in left.cubes() {
        result = union_covers(result, cb_dsharp(cube, right, structure)?)?;
    }

    Ok(result)
}

pub fn cb1_dsharp(
    cover: &SharpCover,
    cube: &SharpCube,
    structure: &SharpStructure,
) -> SharpResult<SharpCover> {
    ensure_cover_shape(cover, structure)?;
    ensure_cube_shape(cube, structure)?;

    let mut result = SharpCover::empty(structure.set_size());
    for row in cover.cubes() {
        result = union_covers(result, dsharp(row, cube, structure)?)?;
    }

    Ok(result)
}

pub fn cb_dsharp(
    cube: &SharpCube,
    cover: &SharpCover,
    structure: &SharpStructure,
) -> SharpResult<SharpCover> {
    ensure_cube_shape(cube, structure)?;
    ensure_cover_shape(cover, structure)?;

    if cover.is_empty() {
        return Ok(SharpCover::singleton(cube.clone()));
    }

    let mut result = SharpCover::singleton(cube.clone());
    for row in cover.cubes() {
        result = cb1_dsharp(&result, row, structure)?;
    }

    Ok(result)
}

pub fn dsharp(
    left: &SharpCube,
    right: &SharpCube,
    structure: &SharpStructure,
) -> SharpResult<SharpCover> {
    ensure_cube_shape(left, structure)?;
    ensure_cube_shape(right, structure)?;

    if !cubes_intersect(left, right, structure) {
        return Ok(SharpCover::singleton(left.clone()));
    }

    let diff = cube_difference(left, right);
    let and = cube_intersection(left, right);
    let mut covered_mask = SharpCube::empty(structure.set_size());
    let mut result = SharpCover::empty(structure.set_size());

    for variable in structure.variables() {
        if !diff.has_part_in_variable(*variable) {
            for part in variable.parts() {
                covered_mask.insert(part)?;
            }

            continue;
        }

        let coordinate_diff = cube_intersection_with_variable(&diff, *variable);
        let previous_intersection = cube_intersection(&and, &covered_mask);
        let mut next_mask = covered_mask.clone();
        for part in variable.parts() {
            next_mask.insert(part)?;
        }

        let future_left = cube_difference(left, &next_mask);
        result.push(cube_union(
            &cube_union(&coordinate_diff, &previous_intersection),
            &future_left,
        ))?;

        covered_mask = next_mask;
    }

    Ok(result)
}

pub fn cv_intersect(
    left: &SharpCover,
    right: &SharpCover,
    structure: &SharpStructure,
) -> SharpResult<SharpCover> {
    ensure_cover_shape(left, structure)?;
    ensure_cover_shape(right, structure)?;

    let mut result = SharpCover::empty(structure.set_size());
    for left_cube in left.cubes() {
        for right_cube in right.cubes() {
            if cubes_intersect(left_cube, right_cube, structure) {
                result.push(cube_intersection(left_cube, right_cube))?;
            }
        }
    }

    contain_cover(result)
}

fn cb_recur_sharp(
    cube: &SharpCube,
    cover: &SharpCover,
    first: usize,
    last: usize,
    structure: &SharpStructure,
) -> SharpResult<SharpCover> {
    if first == last {
        return sharp(cube, &cover.cubes()[first], structure);
    }

    let middle = (first + last) / 2;
    let left = cb_recur_sharp(cube, cover, first, middle, structure)?;
    let right = cb_recur_sharp(cube, cover, middle + 1, last, structure)?;
    cv_intersect(&left, &right, structure)
}

fn append_covers(mut left: SharpCover, right: SharpCover) -> SharpResult<SharpCover> {
    ensure_same_set_size(left.set_size(), right.set_size())?;
    left.cubes.extend(right.cubes);
    Ok(left)
}

fn union_covers(left: SharpCover, right: SharpCover) -> SharpResult<SharpCover> {
    contain_cover(append_covers(left, right)?)
}

fn contain_cover(cover: SharpCover) -> SharpResult<SharpCover> {
    let mut cubes = cover.cubes;
    cubes.sort_by(|left, right| {
        right
            .parts()
            .len()
            .cmp(&left.parts().len())
            .then_with(|| left.parts().cmp(right.parts()))
    });
    cubes.dedup();

    let original = cubes.clone();
    cubes.retain(|cube| {
        !original.iter().any(|candidate| {
            candidate != cube
                && candidate.parts().len() >= cube.parts().len()
                && candidate.contains_all_parts_of(cube)
        })
    });

    Ok(SharpCover {
        set_size: cover.set_size,
        cubes,
    })
}

fn cubes_intersect(left: &SharpCube, right: &SharpCube, structure: &SharpStructure) -> bool {
    structure
        .variables()
        .iter()
        .all(|variable| left.intersects_variable_with(right, *variable))
}

fn cube_intersection(left: &SharpCube, right: &SharpCube) -> SharpCube {
    SharpCube {
        set_size: left.set_size(),
        parts: left.parts().intersection(right.parts()).copied().collect(),
    }
}

fn cube_union(left: &SharpCube, right: &SharpCube) -> SharpCube {
    SharpCube {
        set_size: left.set_size(),
        parts: left.parts().union(right.parts()).copied().collect(),
    }
}

fn cube_difference(left: &SharpCube, right: &SharpCube) -> SharpCube {
    SharpCube {
        set_size: left.set_size(),
        parts: left.parts().difference(right.parts()).copied().collect(),
    }
}

fn cube_intersection_with_variable(cube: &SharpCube, variable: SharpVariable) -> SharpCube {
    SharpCube {
        set_size: cube.set_size(),
        parts: variable
            .parts()
            .filter(|part| cube.contains(*part))
            .collect(),
    }
}

fn cube_difference_with_variable(cube: &SharpCube, variable: SharpVariable) -> SharpCube {
    SharpCube {
        set_size: cube.set_size(),
        parts: cube
            .parts()
            .iter()
            .copied()
            .filter(|part| !variable.parts().any(|mask_part| mask_part == *part))
            .collect(),
    }
}

fn ensure_cover_shape(cover: &SharpCover, structure: &SharpStructure) -> SharpResult<()> {
    ensure_same_set_size(cover.set_size(), structure.set_size())?;
    for cube in cover.cubes() {
        ensure_cube_shape(cube, structure)?;
    }

    Ok(())
}

fn ensure_cube_shape(cube: &SharpCube, structure: &SharpStructure) -> SharpResult<()> {
    ensure_same_set_size(cube.set_size(), structure.set_size())
}

fn ensure_same_set_size(left: usize, right: usize) -> SharpResult<()> {
    if left != right {
        return Err(SharpError::SetSizeMismatch { left, right });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structure() -> SharpStructure {
        SharpStructure::new(
            6,
            [
                SharpVariable::new(0, 1),
                SharpVariable::new(2, 3),
                SharpVariable::new(4, 5),
            ],
        )
        .unwrap()
    }

    fn cube(parts: &[usize]) -> SharpCube {
        SharpCube::from_parts(6, parts.iter().copied()).unwrap()
    }

    fn cover(rows: &[&[usize]]) -> SharpCover {
        SharpCover::new(6, rows.iter().map(|parts| cube(parts))).unwrap()
    }

    fn rows(cover: SharpCover) -> Vec<Vec<usize>> {
        cover
            .cubes()
            .iter()
            .map(|cube| cube.parts().iter().copied().collect())
            .collect()
    }

    #[test]
    fn sharp_keeps_disjoint_cube_unchanged() {
        let structure = structure();
        let left = cube(&[0, 2, 4]);
        let right = cube(&[1, 2, 4]);

        assert_eq!(
            rows(sharp(&left, &right, &structure).unwrap()),
            vec![vec![0, 2, 4]]
        );
    }

    #[test]
    fn sharp_splits_intersecting_cube_by_variable_difference() {
        let structure = structure();
        let left = cube(&[0, 1, 2, 4]);
        let right = cube(&[1, 2, 4, 5]);

        assert_eq!(
            rows(sharp(&left, &right, &structure).unwrap()),
            vec![vec![0, 2, 4]]
        );
    }

    #[test]
    fn cube_cover_sharp_intersects_recursive_halves() {
        let structure = structure();
        let left = cube(&[0, 1, 2, 3, 4]);
        let blockers = cover(&[&[1, 2, 4], &[0, 3, 4]]);

        assert_eq!(
            rows(cb_sharp(&left, &blockers, &structure).unwrap()),
            vec![vec![0, 2, 4], vec![1, 3, 4]]
        );
    }

    #[test]
    fn dsharp_preserves_disjointness_between_generated_rows() {
        let structure = structure();
        let left = cube(&[0, 1, 2, 3, 4]);
        let right = cube(&[1, 2, 4]);

        assert_eq!(
            rows(dsharp(&left, &right, &structure).unwrap()),
            vec![vec![0, 2, 3, 4], vec![1, 3, 4]]
        );
    }

    #[test]
    fn make_disjoint_subtracts_preceding_cubes_from_later_cubes() {
        let structure = structure();
        let source = cover(&[&[0, 1, 2, 4], &[1, 2, 3, 4]]);

        assert_eq!(
            rows(make_disjoint(&source, &structure).unwrap()),
            vec![vec![0, 1, 2, 4], vec![1, 3, 4]]
        );
    }

    #[test]
    fn cover_intersection_applies_containment() {
        let structure = structure();
        let left = cover(&[&[0, 1, 2, 4], &[0, 2, 4]]);
        let right = cover(&[&[0, 1, 2, 3, 4]]);

        assert_eq!(
            rows(cv_intersect(&left, &right, &structure).unwrap()),
            vec![vec![0, 1, 2, 4]]
        );
    }

    #[test]
    fn rejects_mismatched_cube_and_structure_sizes() {
        let structure = structure();
        let mismatched = SharpCube::from_parts(7, [0, 2, 4]).unwrap();

        assert_eq!(
            sharp(&mismatched, &cube(&[0, 2, 4]), &structure).unwrap_err(),
            SharpError::SetSizeMismatch { left: 7, right: 6 }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present() {
        let source = include_str!("sharp.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
