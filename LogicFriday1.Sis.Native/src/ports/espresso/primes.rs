//! Native Rust port of Espresso prime generation by consensus.
//!
//! The legacy routine recursively cofactors a cover, handles tautology and
//! unate short-cuts, and merges distance-one cofactors by consensus. This port
//! keeps those operations on owned Rust values and leaves interop boundaries to
//! callers above the SIS port.

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
    pub fn new(variables: impl IntoIterator<Item = Variable>) -> PrimesResult<Self> {
        let variables = variables.into_iter().collect::<Vec<_>>();
        let mut next_part = 0;
        for variable in &variables {
            if variable.first_part > variable.last_part || variable.first_part < next_part {
                return Err(PrimesError::InvalidVariable {
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
        Cube::full(self.part_count)
    }

    fn validate_part(&self, part: usize) -> PrimesResult<()> {
        if part >= self.part_count {
            return Err(PrimesError::PartOutOfRange {
                part,
                part_count: self.part_count,
            });
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Cube {
    parts: BTreeSet<usize>,
}

impl Cube {
    pub fn empty() -> Self {
        Self {
            parts: BTreeSet::new(),
        }
    }

    pub fn full(part_count: usize) -> Self {
        Self {
            parts: (0..part_count).collect(),
        }
    }

    pub fn from_parts(parts: impl IntoIterator<Item = usize>) -> Self {
        Self {
            parts: parts.into_iter().collect(),
        }
    }

    pub fn parts(&self) -> &BTreeSet<usize> {
        &self.parts
    }

    pub fn contains(&self, part: usize) -> bool {
        self.parts.contains(&part)
    }

    pub fn insert(&mut self, part: usize) -> bool {
        self.parts.insert(part)
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

    pub fn difference(&self, other: &Self) -> Self {
        Self {
            parts: self.parts.difference(&other.parts).copied().collect(),
        }
    }

    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.parts.is_subset(&other.parts)
    }

    pub fn is_full_for(&self, structure: &CubeStructure) -> bool {
        self.parts.len() == structure.part_count()
            && self
                .parts
                .iter()
                .copied()
                .all(|part| part < structure.part_count())
    }

    pub fn intersects_variable(self: &Self, variable: Variable, mask: &Self) -> bool {
        variable
            .parts()
            .any(|part| mask.contains(part) && self.contains(part))
    }

    pub fn valid_for(&self, structure: &CubeStructure) -> bool {
        structure
            .variables()
            .iter()
            .copied()
            .all(|variable| variable.parts().any(|part| self.contains(part)))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover {
    cubes: Vec<Cube>,
}

impl Cover {
    pub fn new(cubes: impl IntoIterator<Item = Cube>) -> Self {
        Self {
            cubes: cubes.into_iter().collect(),
        }
    }

    pub fn from_rows<I, R>(structure: &CubeStructure, rows: I) -> PrimesResult<Self>
    where
        I: IntoIterator<Item = R>,
        R: IntoIterator<Item = usize>,
    {
        let mut cubes = Vec::new();
        for row in rows {
            let cube = Cube::from_parts(row);
            for part in cube.parts() {
                structure.validate_part(*part)?;
            }

            cubes.push(cube);
        }

        Ok(Self::new(cubes))
    }

    pub fn empty() -> Self {
        Self { cubes: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    pub fn into_cubes(self) -> Vec<Cube> {
        self.cubes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrimesError {
    InvalidVariable { first_part: usize, last_part: usize },
    PartOutOfRange { part: usize, part_count: usize },
}

impl fmt::Display for PrimesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
        }
    }
}

impl Error for PrimesError {}

pub type PrimesResult<T> = Result<T, PrimesError>;

#[derive(Clone, Debug)]
struct CubeList {
    cofactor: Cube,
    cubes: Vec<Cube>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CountData {
    vars_active: usize,
    vars_unate: usize,
    best_variable: usize,
}

pub fn primes_consensus(structure: &CubeStructure, cover: Cover) -> Cover {
    let list = CubeList {
        cofactor: Cube::empty(),
        cubes: cover.into_cubes(),
    };

    primes_consensus_list(structure, list)
}

fn primes_consensus_list(structure: &CubeStructure, list: CubeList) -> Cover {
    if let Some(cover) = primes_consensus_special_cases(structure, &list) {
        return cover;
    }

    let count_data = massive_count(structure, &list);
    let (left_cofactor, right_cofactor, best_variable) =
        binate_split_select(structure, &list, count_data.best_variable);
    let left = primes_consensus_list(
        structure,
        single_variable_cofactor(structure, &list, &left_cofactor, best_variable),
    );
    let right = primes_consensus_list(
        structure,
        single_variable_cofactor(structure, &list, &right_cofactor, best_variable),
    );

    primes_consensus_merge(structure, left, right, &left_cofactor, &right_cofactor)
}

fn primes_consensus_special_cases(structure: &CubeStructure, list: &CubeList) -> Option<Cover> {
    if list.cubes.is_empty() {
        return Some(Cover::empty());
    }

    if list.cubes.len() == 1 {
        return Some(Cover::new([list.cofactor.union(&list.cubes[0])]));
    }

    if list
        .cubes
        .iter()
        .any(|cube| cube.union(&list.cofactor).is_full_for(structure))
    {
        return Some(Cover::new([structure.full_cube()]));
    }

    let ceil = list
        .cubes
        .iter()
        .fold(list.cofactor.clone(), |accumulator, cube| {
            accumulator.union(cube)
        });
    let full = structure.full_cube();
    if !ceil.is_full_for(structure) {
        let factored = full.difference(&ceil);
        let mut factored_list = list.clone();
        factored_list.cofactor = factored_list.cofactor.union(&factored);
        let cover = primes_consensus_list(structure, factored_list);
        return Some(Cover::new(
            cover
                .into_cubes()
                .into_iter()
                .map(|cube| cube.intersection(&ceil)),
        ));
    }

    let count_data = massive_count(structure, list);
    if count_data.vars_active == 1 {
        Some(Cover::new([structure.full_cube()]))
    } else if count_data.vars_unate == count_data.vars_active {
        Some(contain(cube_unlist(list)))
    } else {
        None
    }
}

fn massive_count(structure: &CubeStructure, list: &CubeList) -> CountData {
    let full = structure.full_cube();
    let mut part_zeros = vec![0usize; structure.part_count()];
    for cube in &list.cubes {
        for part in full.difference(&cube.union(&list.cofactor)).parts() {
            part_zeros[*part] += 1;
        }
    }

    let mut vars_active = 0;
    let mut vars_unate = 0;
    let mut best_variable = 0;
    let mut most_active = 0;
    let mut most_zero = 0;
    let mut most_balanced = usize::MAX;

    for (index, variable) in structure.variables().iter().copied().enumerate() {
        let counts = variable
            .parts()
            .map(|part| part_zeros[part])
            .collect::<Vec<_>>();
        let active = counts.iter().filter(|count| **count > 0).count();
        let zero_count = counts.iter().sum::<usize>();
        let max_active = counts.iter().copied().max().unwrap_or(0);

        if active > most_active
            || (active == most_active
                && (zero_count > most_zero
                    || (zero_count == most_zero && max_active < most_balanced)))
        {
            best_variable = index;
            most_active = active;
            most_zero = zero_count;
            most_balanced = max_active;
        }

        vars_active += usize::from(active > 0);
        vars_unate += usize::from(active == 1);
    }

    CountData {
        vars_active,
        vars_unate,
        best_variable,
    }
}

fn binate_split_select(
    structure: &CubeStructure,
    list: &CubeList,
    best_variable: usize,
) -> (Cube, Cube, usize) {
    let variable = structure
        .variable(best_variable)
        .expect("best variable selected from structure");
    let variable_parts = Cube::from_parts(variable.parts());
    let base = structure.full_cube().difference(&variable_parts);
    let remaining = variable
        .parts()
        .filter(|part| !list.cofactor.contains(*part))
        .collect::<Vec<_>>();
    let half = remaining.len() / 2;

    let mut left = base.clone();
    for part in remaining.iter().take(half) {
        left.insert(*part);
    }

    let mut right = base;
    for part in remaining.iter().skip(half) {
        right.insert(*part);
    }

    (left, right, best_variable)
}

fn single_variable_cofactor(
    structure: &CubeStructure,
    list: &CubeList,
    cofactor: &Cube,
    variable_index: usize,
) -> CubeList {
    let variable = structure
        .variable(variable_index)
        .expect("cofactor variable selected from structure");
    let full = structure.full_cube();
    let mask = Cube::from_parts(
        variable
            .parts()
            .filter(|part| cofactor.contains(*part))
            .collect::<Vec<_>>(),
    );

    CubeList {
        cofactor: list.cofactor.union(&full.difference(cofactor)),
        cubes: list
            .cubes
            .iter()
            .filter(|cube| cube.intersects_variable(variable, &mask))
            .cloned()
            .collect(),
    }
}

fn cube_unlist(list: &CubeList) -> Cover {
    Cover::new(
        list.cubes
            .iter()
            .map(|cube| cube.union(&list.cofactor))
            .collect::<Vec<_>>(),
    )
}

fn primes_consensus_merge(
    structure: &CubeStructure,
    left: Cover,
    right: Cover,
    left_cofactor: &Cube,
    right_cofactor: &Cube,
) -> Cover {
    let left = and_with_cofactor(structure, left, left_cofactor);
    let right = and_with_cofactor(structure, right, right_cofactor);
    let mut merged = Vec::new();
    merged.extend(left.cubes().iter().cloned());
    merged.extend(right.cubes().iter().cloned());

    for left_cube in left.cubes() {
        for right_cube in right.cubes() {
            if cube_distance_at_most_one(structure, left_cube, right_cube) == 1 {
                merged.push(consensus(structure, left_cube, right_cube));
            }
        }
    }

    contain(Cover::new(merged))
}

fn and_with_cofactor(structure: &CubeStructure, cover: Cover, cofactor: &Cube) -> Cover {
    Cover::new(
        cover
            .into_cubes()
            .into_iter()
            .map(|cube| cube.intersection(cofactor))
            .filter(|cube| cube.valid_for(structure))
            .collect::<Vec<_>>(),
    )
}

fn cube_distance_at_most_one(structure: &CubeStructure, left: &Cube, right: &Cube) -> usize {
    let mut distance = 0;
    for variable in structure.variables().iter().copied() {
        if !variable
            .parts()
            .any(|part| left.contains(part) && right.contains(part))
        {
            distance += 1;
            if distance > 1 {
                return 2;
            }
        }
    }

    distance
}

fn consensus(structure: &CubeStructure, left: &Cube, right: &Cube) -> Cube {
    let mut result = Cube::empty();
    for variable in structure.variables().iter().copied() {
        let intersection = variable
            .parts()
            .filter(|part| left.contains(*part) && right.contains(*part))
            .collect::<Vec<_>>();
        let parts = if intersection.is_empty() {
            variable
                .parts()
                .filter(|part| left.contains(*part) || right.contains(*part))
                .collect::<Vec<_>>()
        } else {
            intersection
        };

        for part in parts {
            result.insert(part);
        }
    }

    result
}

fn contain(cover: Cover) -> Cover {
    let mut cubes = cover.into_cubes();
    cubes.sort_by_key(|cube| (usize::MAX - cube.parts().len(), cube.parts().clone()));
    cubes.dedup();
    let original = cubes.clone();
    cubes.retain(|cube| {
        !original
            .iter()
            .any(|candidate| candidate != cube && cube.is_subset_of(candidate))
    });
    cubes.sort_by_key(|cube| (usize::MAX - cube.parts().len(), cube.parts().clone()));

    Cover::new(cubes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structure() -> CubeStructure {
        CubeStructure::new([
            Variable::new(0, 1),
            Variable::new(2, 3),
            Variable::new(4, 5),
        ])
        .unwrap()
    }

    fn cover(rows: &[&[usize]]) -> Cover {
        Cover::from_rows(&structure(), rows.iter().map(|row| row.iter().copied())).unwrap()
    }

    fn rows(cover: Cover) -> Vec<BTreeSet<usize>> {
        cover
            .into_cubes()
            .into_iter()
            .map(|cube| cube.parts().clone())
            .collect()
    }

    fn set(parts: &[usize]) -> BTreeSet<usize> {
        parts.iter().copied().collect()
    }

    #[test]
    fn empty_cover_has_no_primes() {
        let result = primes_consensus(&structure(), Cover::empty());

        assert!(result.is_empty());
    }

    #[test]
    fn single_cube_is_returned_as_the_only_prime() {
        let structure = structure();
        let result = primes_consensus(&structure, cover(&[&[0, 2, 4]]));

        assert_eq!(rows(result), vec![set(&[0, 2, 4])]);
    }

    #[test]
    fn tautology_row_returns_full_cube() {
        let structure = structure();
        let result = primes_consensus(&structure, cover(&[&[0, 1, 2, 3, 4, 5], &[0, 2, 4]]));

        assert_eq!(rows(result), vec![set(&[0, 1, 2, 3, 4, 5])]);
    }

    #[test]
    fn unate_cover_is_unlisted_and_contained() {
        let structure = structure();
        let result = primes_consensus(&structure, cover(&[&[0, 2, 4], &[0, 2, 4, 5]]));

        assert_eq!(rows(result), vec![set(&[0, 2, 4, 5])]);
    }

    #[test]
    fn consensus_merges_distance_one_cubes_into_a_prime() {
        let structure = structure();
        let result = primes_consensus(&structure, cover(&[&[0, 2, 4], &[1, 2, 4]]));

        assert_eq!(rows(result), vec![set(&[0, 1, 2, 4])]);
    }

    #[test]
    fn invalid_cube_part_is_rejected() {
        let error = Cover::from_rows(&structure(), [vec![0, 7]]).unwrap_err();

        assert_eq!(
            error,
            PrimesError::PartOutOfRange {
                part: 7,
                part_count: 6,
            }
        );
    }

    #[test]
    fn source_contains_no_dependency_metadata_or_c_abi_shims() {
        let source = include_str!("primes.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
    }
}
