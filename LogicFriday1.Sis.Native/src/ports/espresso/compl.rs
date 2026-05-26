//! Native Rust complement and lightweight simplification for Espresso covers.
//!
//! Cubes use Espresso's multiple-valued interpretation: each variable owns a
//! contiguous range of parts, and a cube covers an assignment when the
//! assignment's selected part is present for every variable. The complement
//! routines below expose owned Rust data structures instead of per-file C ABI
//! entry points.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComplementVariable
{
    first_part: usize,
    last_part: usize,
}

impl ComplementVariable
{
    pub const fn new(first_part: usize, last_part: usize) -> Self
    {
        Self {
            first_part,
            last_part,
        }
    }

    pub const fn first_part(self) -> usize
    {
        self.first_part
    }

    pub const fn last_part(self) -> usize
    {
        self.last_part
    }

    pub const fn part_count(self) -> usize
    {
        self.last_part - self.first_part + 1
    }

    pub fn parts(self) -> impl Iterator<Item = usize>
    {
        self.first_part..=self.last_part
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplementStructure
{
    part_count: usize,
    variables: Vec<ComplementVariable>,
}

impl ComplementStructure
{
    pub fn new(
        part_count: usize,
        variables: impl IntoIterator<Item = ComplementVariable>,
    ) -> ComplementResult<Self>
    {
        let variables = variables.into_iter().collect::<Vec<_>>();
        let mut previous_last = None;

        for variable in &variables
        {
            if variable.first_part() > variable.last_part()
                || variable.last_part() >= part_count
            {
                return Err(ComplementError::InvalidVariable {
                    first_part: variable.first_part(),
                    last_part: variable.last_part(),
                    part_count,
                });
            }

            if let Some(previous_last) = previous_last
            {
                if variable.first_part() <= previous_last
                {
                    return Err(ComplementError::OverlappingVariablePart {
                        part: variable.first_part(),
                    });
                }
            }

            previous_last = Some(variable.last_part());
        }

        Ok(Self {
            part_count,
            variables,
        })
    }

    pub fn part_count(&self) -> usize
    {
        self.part_count
    }

    pub fn variables(&self) -> &[ComplementVariable]
    {
        &self.variables
    }

    pub fn variable_count(&self) -> usize
    {
        self.variables.len()
    }

    pub fn full_cube(&self) -> ComplementCube
    {
        ComplementCube::from_parts_unchecked(self.part_count, 0..self.part_count)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ComplementCube
{
    part_count: usize,
    parts: BTreeSet<usize>,
}

impl ComplementCube
{
    pub fn empty(part_count: usize) -> Self
    {
        Self {
            part_count,
            parts: BTreeSet::new(),
        }
    }

    pub fn from_parts(
        part_count: usize,
        parts: impl IntoIterator<Item = usize>,
    ) -> ComplementResult<Self>
    {
        let mut cube = Self::empty(part_count);
        for part in parts
        {
            cube.insert(part)?;
        }

        Ok(cube)
    }

    pub fn part_count(&self) -> usize
    {
        self.part_count
    }

    pub fn parts(&self) -> &BTreeSet<usize>
    {
        &self.parts
    }

    pub fn contains(&self, part: usize) -> bool
    {
        self.parts.contains(&part)
    }

    pub fn insert(&mut self, part: usize) -> ComplementResult<bool>
    {
        if part >= self.part_count
        {
            return Err(ComplementError::PartOutOfRange {
                part,
                part_count: self.part_count,
            });
        }

        Ok(self.parts.insert(part))
    }

    pub fn covers_assignment(&self, assignment: &[usize], structure: &ComplementStructure) -> bool
    {
        assignment.len() == structure.variable_count()
            && assignment.iter().copied().all(|part| self.contains(part))
    }

    fn from_parts_unchecked(
        part_count: usize,
        parts: impl IntoIterator<Item = usize>,
    ) -> Self
    {
        Self {
            part_count,
            parts: parts.into_iter().collect(),
        }
    }

    fn is_subset_of(&self, other: &Self) -> bool
    {
        self.parts.is_subset(&other.parts)
    }

}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplementCover
{
    part_count: usize,
    cubes: Vec<ComplementCube>,
}

impl ComplementCover
{
    pub fn new(
        part_count: usize,
        cubes: impl IntoIterator<Item = ComplementCube>,
    ) -> ComplementResult<Self>
    {
        let cubes = cubes.into_iter().collect::<Vec<_>>();
        for cube in &cubes
        {
            ensure_same_part_count(cube.part_count(), part_count)?;
        }

        Ok(Self {
            part_count,
            cubes,
        })
    }

    pub fn empty(part_count: usize) -> Self
    {
        Self {
            part_count,
            cubes: Vec::new(),
        }
    }

    pub fn singleton(cube: ComplementCube) -> Self
    {
        Self {
            part_count: cube.part_count(),
            cubes: vec![cube],
        }
    }

    pub fn part_count(&self) -> usize
    {
        self.part_count
    }

    pub fn cubes(&self) -> &[ComplementCube]
    {
        &self.cubes
    }

    pub fn len(&self) -> usize
    {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.cubes.is_empty()
    }

    pub fn push(&mut self, cube: ComplementCube) -> ComplementResult<()>
    {
        ensure_same_part_count(cube.part_count(), self.part_count)?;
        self.cubes.push(cube);
        Ok(())
    }

    pub fn covers_assignment(
        &self,
        assignment: &[usize],
        structure: &ComplementStructure,
    ) -> bool
    {
        self.cubes
            .iter()
            .any(|cube| cube.covers_assignment(assignment, structure))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComplementError
{
    InvalidVariable {
        first_part: usize,
        last_part: usize,
        part_count: usize,
    },
    OverlappingVariablePart {
        part: usize,
    },
    PartOutOfRange {
        part: usize,
        part_count: usize,
    },
    PartCountMismatch {
        left: usize,
        right: usize,
    },
    NonOrthogonalCovers,
}

impl fmt::Display for ComplementError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::InvalidVariable {
                first_part,
                last_part,
                part_count,
            } => write!(
                formatter,
                "variable part range {first_part}..={last_part} is invalid for part count {part_count}"
            ),
            Self::OverlappingVariablePart { part } => {
                write!(formatter, "part {part} appears in more than one variable")
            }
            Self::PartOutOfRange { part, part_count } => {
                write!(formatter, "part {part} is outside part count {part_count}")
            }
            Self::PartCountMismatch { left, right } => {
                write!(formatter, "part counts differ: {left} != {right}")
            }
            Self::NonOrthogonalCovers => {
                write!(formatter, "cover and complement intersect")
            }
        }
    }
}

impl Error for ComplementError {}

pub type ComplementResult<T> = Result<T, ComplementError>;

pub fn complement(
    cover: &ComplementCover,
    structure: &ComplementStructure,
) -> ComplementResult<ComplementCover>
{
    ensure_cover_shape(cover, structure)?;

    if cover.is_empty()
    {
        return Ok(ComplementCover::singleton(structure.full_cube()));
    }

    if cover
        .cubes()
        .iter()
        .any(|cube| is_full_row(cube, structure))
    {
        return Ok(ComplementCover::empty(structure.part_count()));
    }

    if cover.len() == 1
    {
        return complement_cube(&cover.cubes()[0], structure);
    }

    let mut result = ComplementCover::empty(structure.part_count());
    enumerate_assignments(structure, |assignment| {
        if !cover.covers_assignment(assignment, structure)
        {
            result.push(ComplementCube::from_parts_unchecked(
                structure.part_count(),
                assignment.iter().copied(),
            ))?;
        }

        Ok(())
    })?;

    contain(result)
}

pub fn simplify(
    cover: &ComplementCover,
    structure: &ComplementStructure,
) -> ComplementResult<ComplementCover>
{
    ensure_cover_shape(cover, structure)?;

    if cover.is_empty()
    {
        return Ok(ComplementCover::empty(structure.part_count()));
    }

    if cover
        .cubes()
        .iter()
        .any(|cube| is_full_row(cube, structure))
    {
        return Ok(ComplementCover::singleton(structure.full_cube()));
    }

    contain(cover.clone())
}

pub fn simplify_and_complement(
    cover: &ComplementCover,
    structure: &ComplementStructure,
) -> ComplementResult<(ComplementCover, ComplementCover)>
{
    Ok((simplify(cover, structure)?, complement(cover, structure)?))
}

pub fn complement_cube(
    cube: &ComplementCube,
    structure: &ComplementStructure,
) -> ComplementResult<ComplementCover>
{
    ensure_cube_shape(cube, structure)?;

    let mut result = ComplementCover::empty(structure.part_count());
    for variable in structure.variables()
    {
        let missing_parts = variable
            .parts()
            .filter(|part| !cube.contains(*part))
            .collect::<Vec<_>>();

        if missing_parts.is_empty()
        {
            continue;
        }

        let mut row = structure.full_cube();
        for part in variable.parts()
        {
            row.parts.remove(&part);
        }

        for part in missing_parts
        {
            row.parts.insert(part);
        }

        result.push(row)?;
    }

    Ok(result)
}

pub fn verify_complement(
    cover: &ComplementCover,
    complement_cover: &ComplementCover,
    structure: &ComplementStructure,
) -> ComplementResult<bool>
{
    ensure_cover_shape(cover, structure)?;
    ensure_cover_shape(complement_cover, structure)?;

    let mut complete = true;
    enumerate_assignments(structure, |assignment| {
        let in_cover = cover.covers_assignment(assignment, structure);
        let in_complement = complement_cover.covers_assignment(assignment, structure);

        if in_cover && in_complement
        {
            return Err(ComplementError::NonOrthogonalCovers);
        }

        complete &= in_cover || in_complement;
        Ok(())
    })?;

    Ok(complete)
}

fn contain(cover: ComplementCover) -> ComplementResult<ComplementCover>
{
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
                && cube.is_subset_of(candidate)
                && candidate.part_count() == cube.part_count()
        })
    });

    Ok(ComplementCover {
        part_count: cover.part_count,
        cubes,
    })
}

fn is_full_row(cube: &ComplementCube, structure: &ComplementStructure) -> bool
{
    structure
        .variables()
        .iter()
        .copied()
        .all(|variable| variable.parts().all(|part| cube.contains(part)))
}

fn enumerate_assignments(
    structure: &ComplementStructure,
    mut visit: impl FnMut(&[usize]) -> ComplementResult<()>,
) -> ComplementResult<()>
{
    let mut assignment = Vec::with_capacity(structure.variable_count());
    enumerate_assignment_suffix(structure, 0, &mut assignment, &mut visit)
}

fn enumerate_assignment_suffix(
    structure: &ComplementStructure,
    variable_index: usize,
    assignment: &mut Vec<usize>,
    visit: &mut impl FnMut(&[usize]) -> ComplementResult<()>,
) -> ComplementResult<()>
{
    if variable_index == structure.variable_count()
    {
        return visit(assignment);
    }

    for part in structure.variables()[variable_index].parts()
    {
        assignment.push(part);
        enumerate_assignment_suffix(structure, variable_index + 1, assignment, visit)?;
        assignment.pop();
    }

    Ok(())
}

fn ensure_cover_shape(
    cover: &ComplementCover,
    structure: &ComplementStructure,
) -> ComplementResult<()>
{
    ensure_same_part_count(cover.part_count(), structure.part_count())?;
    for cube in cover.cubes()
    {
        ensure_cube_shape(cube, structure)?;
    }

    Ok(())
}

fn ensure_cube_shape(
    cube: &ComplementCube,
    structure: &ComplementStructure,
) -> ComplementResult<()>
{
    ensure_same_part_count(cube.part_count(), structure.part_count())?;
    for part in cube.parts()
    {
        if *part >= structure.part_count()
        {
            return Err(ComplementError::PartOutOfRange {
                part: *part,
                part_count: structure.part_count(),
            });
        }
    }

    Ok(())
}

fn ensure_same_part_count(left: usize, right: usize) -> ComplementResult<()>
{
    if left != right
    {
        return Err(ComplementError::PartCountMismatch { left, right });
    }

    Ok(())
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn structure() -> ComplementStructure
    {
        ComplementStructure::new(
            6,
            [
                ComplementVariable::new(0, 1),
                ComplementVariable::new(2, 3),
                ComplementVariable::new(4, 5),
            ],
        )
        .unwrap()
    }

    fn cube(parts: &[usize]) -> ComplementCube
    {
        ComplementCube::from_parts(6, parts.iter().copied()).unwrap()
    }

    fn cover(rows: &[&[usize]]) -> ComplementCover
    {
        ComplementCover::new(6, rows.iter().map(|row| cube(row))).unwrap()
    }

    fn rows(cover: &ComplementCover) -> Vec<Vec<usize>>
    {
        cover
            .cubes()
            .iter()
            .map(|cube| cube.parts().iter().copied().collect())
            .collect()
    }

    #[test]
    fn empty_cover_complements_to_full_cube()
    {
        let structure = structure();
        let result = complement(&ComplementCover::empty(6), &structure).unwrap();

        assert_eq!(rows(&result), vec![vec![0, 1, 2, 3, 4, 5]]);
        assert!(verify_complement(&ComplementCover::empty(6), &result, &structure).unwrap());
    }

    #[test]
    fn full_row_complements_to_empty_cover()
    {
        let structure = structure();
        let source = cover(&[&[0, 1, 2, 3, 4, 5]]);
        let result = complement(&source, &structure).unwrap();

        assert!(result.is_empty());
        assert!(verify_complement(&source, &result, &structure).unwrap());
    }

    #[test]
    fn single_cube_complement_uses_demorgan_rows()
    {
        let structure = structure();
        let source = cover(&[&[0, 2, 4, 5]]);
        let result = complement(&source, &structure).unwrap();

        assert_eq!(
            rows(&result),
            vec![vec![1, 2, 3, 4, 5], vec![0, 1, 3, 4, 5]]
        );
        assert!(verify_complement(&source, &result, &structure).unwrap());
    }

    #[test]
    fn multi_cube_complement_covers_exact_uncovered_assignments()
    {
        let structure = structure();
        let source = cover(&[&[0, 2, 4], &[1, 3, 5]]);
        let result = complement(&source, &structure).unwrap();

        assert_eq!(
            rows(&result),
            vec![
                vec![0, 2, 5],
                vec![0, 3, 4],
                vec![0, 3, 5],
                vec![1, 2, 4],
                vec![1, 2, 5],
                vec![1, 3, 4],
            ]
        );
        assert!(verify_complement(&source, &result, &structure).unwrap());
    }

    #[test]
    fn simplify_removes_contained_rows()
    {
        let structure = structure();
        let source = cover(&[&[0, 2, 4], &[0, 1, 2, 4, 5], &[0, 2, 4]]);
        let result = simplify(&source, &structure).unwrap();

        assert_eq!(rows(&result), vec![vec![0, 1, 2, 4, 5]]);
    }

    #[test]
    fn simplify_and_complement_returns_both_covers()
    {
        let structure = structure();
        let source = cover(&[&[0, 2, 4]]);
        let (simplified, complement_cover) = simplify_and_complement(&source, &structure).unwrap();

        assert_eq!(rows(&simplified), vec![vec![0, 2, 4]]);
        assert!(verify_complement(&simplified, &complement_cover, &structure).unwrap());
    }

    #[test]
    fn rejects_mismatched_part_counts()
    {
        let structure = structure();
        let mismatched = ComplementCover::new(
            7,
            [ComplementCube::from_parts(7, [0, 2, 4]).unwrap()],
        )
        .unwrap();

        assert_eq!(
            complement(&mismatched, &structure).unwrap_err(),
            ComplementError::PartCountMismatch { left: 7, right: 6 }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present()
    {
        let source = include_str!("compl.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
    }
}
