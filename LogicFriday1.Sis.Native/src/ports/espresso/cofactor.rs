//! Native Rust cofactoring utilities for Espresso-style covers.
//!
//! Espresso represents a cube as the selected parts for every variable. A
//! cofactored cube list keeps the accumulated cofactor separately from the row
//! cubes, then restores it when converting back to a cover.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Variable
{
    first_part: usize,
    last_part: usize,
}

impl Variable
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

    pub fn parts(self) -> impl Iterator<Item = usize>
    {
        self.first_part..=self.last_part
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeStructure
{
    variables: Vec<Variable>,
    part_count: usize,
}

impl CubeStructure
{
    pub fn new(variables: impl IntoIterator<Item = Variable>) -> CofactorResult<Self>
    {
        let variables = variables.into_iter().collect::<Vec<_>>();
        let mut next_part = 0;

        for variable in &variables
        {
            if variable.first_part > variable.last_part || variable.first_part < next_part
            {
                return Err(CofactorError::InvalidVariable {
                    first_part: variable.first_part,
                    last_part: variable.last_part,
                });
            }

            next_part = variable.last_part + 1;
        }

        Ok(Self {
            variables,
            part_count: next_part,
        })
    }

    pub fn variables(&self) -> &[Variable]
    {
        &self.variables
    }

    pub fn variable(&self, index: usize) -> CofactorResult<Variable>
    {
        self.variables
            .get(index)
            .copied()
            .ok_or(CofactorError::VariableOutOfRange {
                variable: index,
                variable_count: self.variables.len(),
            })
    }

    pub fn variable_count(&self) -> usize
    {
        self.variables.len()
    }

    pub fn part_count(&self) -> usize
    {
        self.part_count
    }

    pub fn full_cube(&self) -> Cube
    {
        Cube::full(self.part_count)
    }

    fn validate_cube(&self, cube: &Cube) -> CofactorResult<()>
    {
        for part in cube.parts()
        {
            if *part >= self.part_count
            {
                return Err(CofactorError::PartOutOfRange {
                    part: *part,
                    part_count: self.part_count,
                });
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Cube
{
    parts: BTreeSet<usize>,
}

impl Cube
{
    pub fn empty() -> Self
    {
        Self {
            parts: BTreeSet::new(),
        }
    }

    pub fn full(part_count: usize) -> Self
    {
        Self {
            parts: (0..part_count).collect(),
        }
    }

    pub fn from_parts(parts: impl IntoIterator<Item = usize>) -> Self
    {
        Self {
            parts: parts.into_iter().collect(),
        }
    }

    pub fn parts(&self) -> &BTreeSet<usize>
    {
        &self.parts
    }

    pub fn contains(&self, part: usize) -> bool
    {
        self.parts.contains(&part)
    }

    pub fn insert(&mut self, part: usize) -> bool
    {
        self.parts.insert(part)
    }

    pub fn union(&self, other: &Self) -> Self
    {
        Self {
            parts: self.parts.union(&other.parts).copied().collect(),
        }
    }

    pub fn intersection(&self, other: &Self) -> Self
    {
        Self {
            parts: self.parts.intersection(&other.parts).copied().collect(),
        }
    }

    pub fn difference(&self, other: &Self) -> Self
    {
        Self {
            parts: self.parts.difference(&other.parts).copied().collect(),
        }
    }

    fn intersects_variable(&self, other: &Self, variable: Variable) -> bool
    {
        variable
            .parts()
            .any(|part| self.contains(part) && other.contains(part))
    }

    fn contains_any_in(&self, variable: Variable, mask: &Self) -> bool
    {
        variable
            .parts()
            .any(|part| self.contains(part) && mask.contains(part))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover
{
    cubes: Vec<Cube>,
}

impl Cover
{
    pub fn new(cubes: impl IntoIterator<Item = Cube>) -> Self
    {
        Self {
            cubes: cubes.into_iter().collect(),
        }
    }

    pub fn from_rows<I, R>(structure: &CubeStructure, rows: I) -> CofactorResult<Self>
    where
        I: IntoIterator<Item = R>,
        R: IntoIterator<Item = usize>,
    {
        let cubes = rows
            .into_iter()
            .map(Cube::from_parts)
            .collect::<Vec<_>>();
        let cover = Self::new(cubes);
        validate_cover(structure, &cover)?;

        Ok(cover)
    }

    pub fn cubes(&self) -> &[Cube]
    {
        &self.cubes
    }

    pub fn into_cubes(self) -> Vec<Cube>
    {
        self.cubes
    }

    pub fn len(&self) -> usize
    {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.cubes.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeList
{
    cofactor: Cube,
    cubes: Vec<Cube>,
}

impl CubeList
{
    pub fn new(cofactor: Cube, cubes: impl IntoIterator<Item = Cube>) -> Self
    {
        Self {
            cofactor,
            cubes: cubes.into_iter().collect(),
        }
    }

    pub fn cofactor(&self) -> &Cube
    {
        &self.cofactor
    }

    pub fn cubes(&self) -> &[Cube]
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountData
{
    pub part_zeros: Vec<usize>,
    pub variable_zeros: Vec<usize>,
    pub parts_active: Vec<usize>,
    pub is_unate: Vec<bool>,
    pub vars_active: usize,
    pub vars_unate: usize,
    pub best_variable: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CofactorError
{
    InvalidVariable
    {
        first_part: usize,
        last_part: usize,
    },
    PartOutOfRange
    {
        part: usize,
        part_count: usize,
    },
    VariableOutOfRange
    {
        variable: usize,
        variable_count: usize,
    },
}

impl fmt::Display for CofactorError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::InvalidVariable {
                first_part,
                last_part,
            } => write!(
                formatter,
                "invalid variable part range {first_part}..={last_part}"
            ),
            Self::PartOutOfRange { part, part_count } => {
                write!(formatter, "part {part} is outside 0..{part_count}")
            }
            Self::VariableOutOfRange {
                variable,
                variable_count,
            } => write!(
                formatter,
                "variable {variable} is outside 0..{variable_count}"
            ),
        }
    }
}

impl Error for CofactorError {}

pub type CofactorResult<T> = Result<T, CofactorError>;

pub fn cube1list(structure: &CubeStructure, cover: &Cover) -> CofactorResult<CubeList>
{
    validate_cover(structure, cover)?;

    Ok(CubeList::new(Cube::empty(), cover.cubes().iter().cloned()))
}

pub fn cube2list(
    structure: &CubeStructure,
    left: &Cover,
    right: &Cover,
) -> CofactorResult<CubeList>
{
    validate_cover(structure, left)?;
    validate_cover(structure, right)?;

    let mut cubes = left.cubes().to_vec();
    cubes.extend(right.cubes().iter().cloned());

    Ok(CubeList::new(Cube::empty(), cubes))
}

pub fn cube3list(
    structure: &CubeStructure,
    first: &Cover,
    second: &Cover,
    third: &Cover,
) -> CofactorResult<CubeList>
{
    validate_cover(structure, first)?;
    validate_cover(structure, second)?;
    validate_cover(structure, third)?;

    let mut cubes = first.cubes().to_vec();
    cubes.extend(second.cubes().iter().cloned());
    cubes.extend(third.cubes().iter().cloned());

    Ok(CubeList::new(Cube::empty(), cubes))
}

pub fn cubeunlist(structure: &CubeStructure, list: &CubeList) -> CofactorResult<Cover>
{
    validate_cubelist(structure, list)?;

    Ok(Cover::new(
        list.cubes()
            .iter()
            .map(|cube| cube.union(list.cofactor()))
            .collect::<Vec<_>>(),
    ))
}

pub fn cofactor(
    structure: &CubeStructure,
    list: &CubeList,
    cofactor_cube: &Cube,
) -> CofactorResult<CubeList>
{
    validate_cubelist(structure, list)?;
    structure.validate_cube(cofactor_cube)?;

    let full = structure.full_cube();
    let next_cofactor = list.cofactor().union(&full.difference(cofactor_cube));
    let cubes = list
        .cubes()
        .iter()
        .filter(|cube| cubes_distance_zero(structure, cube, cofactor_cube))
        .cloned()
        .collect::<Vec<_>>();

    Ok(CubeList::new(next_cofactor, cubes))
}

pub fn single_variable_cofactor(
    structure: &CubeStructure,
    list: &CubeList,
    cofactor_cube: &Cube,
    variable_index: usize,
) -> CofactorResult<CubeList>
{
    validate_cubelist(structure, list)?;
    structure.validate_cube(cofactor_cube)?;
    let variable = structure.variable(variable_index)?;
    let full = structure.full_cube();
    let next_cofactor = list.cofactor().union(&full.difference(cofactor_cube));
    let mask = Cube::from_parts(variable.parts().filter(|part| cofactor_cube.contains(*part)));
    let cubes = list
        .cubes()
        .iter()
        .filter(|cube| cube.contains_any_in(variable, &mask))
        .cloned()
        .collect::<Vec<_>>();

    Ok(CubeList::new(next_cofactor, cubes))
}

pub fn massive_count(structure: &CubeStructure, list: &CubeList) -> CofactorResult<CountData>
{
    validate_cubelist(structure, list)?;

    let full = structure.full_cube();
    let mut part_zeros = vec![0; structure.part_count()];
    for cube in list.cubes()
    {
        for part in full.difference(&cube.union(list.cofactor())).parts()
        {
            part_zeros[*part] += 1;
        }
    }

    let mut variable_zeros = vec![0; structure.variable_count()];
    let mut parts_active = vec![0; structure.variable_count()];
    let mut is_unate = vec![false; structure.variable_count()];
    let mut vars_active = 0;
    let mut vars_unate = 0;
    let mut best_variable = None;
    let mut most_active = 0;
    let mut most_zero = 0;
    let mut most_balanced = usize::MAX;

    for (index, variable) in structure.variables().iter().copied().enumerate()
    {
        let counts = variable
            .parts()
            .map(|part| part_zeros[part])
            .collect::<Vec<_>>();
        let active = counts.iter().filter(|count| **count > 0).count();
        let zero_count = counts.iter().sum::<usize>();
        let max_zeros_per_part = counts.iter().copied().max().unwrap_or(0);

        if active > most_active
            || (active == most_active
                && (zero_count > most_zero
                    || (zero_count == most_zero && max_zeros_per_part < most_balanced)))
        {
            best_variable = Some(index);
            most_active = active;
            most_zero = zero_count;
            most_balanced = max_zeros_per_part;
        }

        variable_zeros[index] = zero_count;
        parts_active[index] = active;
        is_unate[index] = active == 1;
        vars_active += usize::from(active > 0);
        vars_unate += usize::from(active == 1);
    }

    Ok(CountData {
        part_zeros,
        variable_zeros,
        parts_active,
        is_unate,
        vars_active,
        vars_unate,
        best_variable,
    })
}

pub fn binate_split_select(
    structure: &CubeStructure,
    list: &CubeList,
    best_variable: usize,
) -> CofactorResult<(Cube, Cube, usize)>
{
    validate_cubelist(structure, list)?;
    let variable = structure.variable(best_variable)?;
    let variable_mask = Cube::from_parts(variable.parts());
    let base = structure.full_cube().difference(&variable_mask);
    let remaining = variable
        .parts()
        .filter(|part| !list.cofactor().contains(*part))
        .collect::<Vec<_>>();
    let half = remaining.len() / 2;

    let mut left = base.clone();
    for part in remaining.iter().take(half)
    {
        left.insert(*part);
    }

    let mut right = base;
    for part in remaining.iter().skip(half)
    {
        right.insert(*part);
    }

    Ok((left, right, best_variable))
}

pub fn simplify_cubelist(structure: &CubeStructure, list: &CubeList) -> CofactorResult<CubeList>
{
    validate_cubelist(structure, list)?;

    let mut cubes = list.cubes().to_vec();
    cubes.sort();
    cubes.dedup();

    Ok(CubeList::new(list.cofactor().clone(), cubes))
}

fn validate_cover(structure: &CubeStructure, cover: &Cover) -> CofactorResult<()>
{
    for cube in cover.cubes()
    {
        structure.validate_cube(cube)?;
    }

    Ok(())
}

fn validate_cubelist(structure: &CubeStructure, list: &CubeList) -> CofactorResult<()>
{
    structure.validate_cube(list.cofactor())?;
    for cube in list.cubes()
    {
        structure.validate_cube(cube)?;
    }

    Ok(())
}

fn cubes_distance_zero(structure: &CubeStructure, left: &Cube, right: &Cube) -> bool
{
    structure
        .variables()
        .iter()
        .all(|variable| left.intersects_variable(right, *variable))
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn structure() -> CubeStructure
    {
        CubeStructure::new([
            Variable::new(0, 1),
            Variable::new(2, 3),
            Variable::new(4, 6),
        ])
        .unwrap()
    }

    fn cube(parts: &[usize]) -> Cube
    {
        Cube::from_parts(parts.iter().copied())
    }

    fn cover(rows: &[&[usize]]) -> Cover
    {
        Cover::new(rows.iter().map(|parts| cube(parts)))
    }

    fn rows(cubes: &[Cube]) -> Vec<Vec<usize>>
    {
        cubes
            .iter()
            .map(|cube| cube.parts().iter().copied().collect())
            .collect()
    }

    #[test]
    fn cofactor_keeps_only_cubes_with_distance_zero()
    {
        let structure = structure();
        let list = cube1list(
            &structure,
            &cover(&[&[0, 2, 4], &[1, 2, 4], &[0, 3, 5], &[0, 2, 5, 6]]),
        )
        .unwrap();
        let result = cofactor(&structure, &list, &cube(&[0, 2, 4, 5])).unwrap();

        assert_eq!(
            rows(result.cubes()),
            vec![vec![0, 2, 4], vec![0, 2, 5, 6]]
        );
        assert_eq!(result.cofactor(), &cube(&[1, 3, 6]));
    }

    #[test]
    fn single_variable_cofactor_filters_on_selected_variable_only()
    {
        let structure = structure();
        let list = cube1list(
            &structure,
            &cover(&[&[0, 2, 4], &[1, 3, 5], &[0, 3, 6], &[1, 2, 6]]),
        )
        .unwrap();
        let result = single_variable_cofactor(&structure, &list, &cube(&[0, 1, 2, 4, 5]), 1)
            .unwrap();

        assert_eq!(rows(result.cubes()), vec![vec![0, 2, 4], vec![1, 2, 6]]);
        assert_eq!(result.cofactor(), &cube(&[3, 6]));
    }

    #[test]
    fn massive_count_counts_missing_parts_after_accumulated_cofactor()
    {
        let structure = structure();
        let list = CubeList::new(
            cube(&[1]),
            [cube(&[0, 2, 4]), cube(&[0, 3, 5]), cube(&[1, 2, 6])],
        );
        let count = massive_count(&structure, &list).unwrap();

        assert_eq!(count.part_zeros, vec![1, 0, 1, 2, 2, 2, 2]);
        assert_eq!(count.variable_zeros, vec![1, 3, 6]);
        assert_eq!(count.parts_active, vec![1, 2, 3]);
        assert_eq!(count.is_unate, vec![true, false, false]);
        assert_eq!(count.vars_active, 3);
        assert_eq!(count.vars_unate, 1);
        assert_eq!(count.best_variable, Some(2));
    }

    #[test]
    fn binate_split_select_partitions_uncofactored_parts_of_best_variable()
    {
        let structure = structure();
        let list = CubeList::new(cube(&[5]), [cube(&[0, 2, 4])]);
        let (left, right, selected) = binate_split_select(&structure, &list, 2).unwrap();

        assert_eq!(selected, 2);
        assert_eq!(left, cube(&[0, 1, 2, 3, 4]));
        assert_eq!(right, cube(&[0, 1, 2, 3, 6]));
    }

    #[test]
    fn cube_lists_roundtrip_through_accumulated_cofactor()
    {
        let structure = structure();
        let left = cover(&[&[0, 2, 4]]);
        let right = cover(&[&[1, 3, 5]]);
        let third = cover(&[&[0, 3, 6]]);
        let list = cube3list(&structure, &left, &right, &third).unwrap();
        let result = cubeunlist(&structure, &CubeList::new(cube(&[6]), list.cubes().to_vec()))
            .unwrap();

        assert_eq!(
            rows(result.cubes()),
            vec![vec![0, 2, 4, 6], vec![1, 3, 5, 6], vec![0, 3, 6]]
        );
    }

    #[test]
    fn simplify_cubelist_sorts_and_deduplicates_rows()
    {
        let structure = structure();
        let list = CubeList::new(
            cube(&[1]),
            [cube(&[1, 3, 5]), cube(&[0, 2, 4]), cube(&[1, 3, 5])],
        );
        let result = simplify_cubelist(&structure, &list).unwrap();

        assert_eq!(result.cofactor(), &cube(&[1]));
        assert_eq!(rows(result.cubes()), vec![vec![0, 2, 4], vec![1, 3, 5]]);
    }

    #[test]
    fn rejects_out_of_range_parts()
    {
        let structure = structure();
        let list = CubeList::new(Cube::empty(), [cube(&[7])]);

        assert_eq!(
            massive_count(&structure, &list).unwrap_err(),
            CofactorError::PartOutOfRange {
                part: 7,
                part_count: 7,
            }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present()
    {
        let source = include_str!("cofactor.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
