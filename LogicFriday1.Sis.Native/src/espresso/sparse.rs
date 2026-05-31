//! Native Rust port of the Espresso sparse-cover cleanup from `sis/espresso/sparse.c`.
//!
//! The C routine alternates sparse-variable reduction with expansion until the
//! cover literal total stops improving. The expansion algorithm belongs to a
//! separate Espresso port, so this module exposes the cleanup driver through an
//! explicit callback and implements the self-contained sparse reduction pass.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SparseOptions {
    pub force_irredundant: bool,
}

impl SparseOptions {
    pub const fn sis_defaults() -> Self {
        Self {
            force_irredundant: true,
        }
    }
}

impl Default for SparseOptions {
    fn default() -> Self {
        Self::sis_defaults()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoverCost {
    pub cubes: usize,
    pub total: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Variable {
    first_part: usize,
    last_part: usize,
    sparse: bool,
}

impl Variable {
    pub const fn new(first_part: usize, last_part: usize, sparse: bool) -> Self {
        Self {
            first_part,
            last_part,
            sparse,
        }
    }

    pub const fn first_part(self) -> usize {
        self.first_part
    }

    pub const fn last_part(self) -> usize {
        self.last_part
    }

    pub const fn is_sparse(self) -> bool {
        self.sparse
    }

    fn parts(self) -> impl Iterator<Item = usize> {
        self.first_part..=self.last_part
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeStructure {
    variables: Vec<Variable>,
}

impl CubeStructure {
    pub fn new(variables: impl IntoIterator<Item = Variable>) -> Self {
        let variables = variables.into_iter().collect::<Vec<_>>();

        for window in variables.windows(2) {
            debug_assert!(window[0].last_part < window[1].first_part);
        }

        Self { variables }
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

    pub fn output_variable_index(&self) -> Option<usize> {
        self.variables.len().checked_sub(1)
    }

    pub fn cost(&self, cover: &Cover) -> CoverCost {
        let total = cover
            .cubes()
            .iter()
            .map(|cube| {
                self.variables
                    .iter()
                    .filter(|variable| !cube.contains_all_parts(**variable))
                    .map(|variable| cube.part_count_for(*variable))
                    .sum::<usize>()
            })
            .sum();

        CoverCost {
            cubes: cover.len(),
            total,
        }
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

    pub fn contains(&self, part: usize) -> bool {
        self.parts.contains(&part)
    }

    pub fn insert(&mut self, part: usize) -> bool {
        self.parts.insert(part)
    }

    pub fn remove(&mut self, part: usize) -> bool {
        self.parts.remove(&part)
    }

    pub fn parts(&self) -> impl Iterator<Item = usize> + '_ {
        self.parts.iter().copied()
    }

    pub fn cofactored_on(&self, variable: Variable, part: usize) -> Self {
        let mut result = Self::from_parts(
            self.parts()
                .filter(|candidate| !variable.parts().any(|mask_part| mask_part == *candidate)),
        );
        result.insert(part);
        result
    }

    pub fn is_disjoint_from(&self, variable: Variable) -> bool {
        !variable.parts().any(|part| self.contains(part))
    }

    pub fn contains_all_parts(&self, variable: Variable) -> bool {
        variable.parts().all(|part| self.contains(part))
    }

    pub fn part_count_for(&self, variable: Variable) -> usize {
        variable.parts().filter(|part| self.contains(*part)).count()
    }

    fn contains_assignment(&self, assignment: &[usize]) -> bool {
        assignment.iter().all(|part| self.contains(*part))
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

    pub fn cubes_mut(&mut self) -> &mut [Cube] {
        &mut self.cubes
    }

    pub fn push(&mut self, cube: Cube) {
        self.cubes.push(cube);
    }

    fn retain_active(&mut self, active: &[bool]) {
        let mut index = 0;
        self.cubes.retain(|_| {
            let keep = active[index];
            index += 1;
            keep
        });
    }
}

pub fn make_sparse_with_expander<E>(
    mut cover: Cover,
    dont_care: &Cover,
    off_set: &Cover,
    structure: &CubeStructure,
    options: SparseOptions,
    mut expand: E,
) -> Cover
where
    E: FnMut(Cover, &Cover) -> Cover,
{
    let mut best_cost = structure.cost(&cover);

    loop {
        cover = mv_reduce(cover, dont_care, structure);
        let cost = structure.cost(&cover);
        if cost.total == best_cost.total {
            break;
        }
        best_cost = cost;

        cover = expand(cover, off_set);
        let cost = structure.cost(&cover);
        if cost.total == best_cost.total {
            break;
        }
        best_cost = cost;

        if !options.force_irredundant {
            break;
        }
    }

    cover
}

pub fn mv_reduce(mut cover: Cover, dont_care: &Cover, structure: &CubeStructure) -> Cover {
    for (variable_index, variable) in structure.variables().iter().copied().enumerate() {
        if !variable.is_sparse() {
            continue;
        }

        for part in variable.parts() {
            let mut mapped_indices = Vec::new();
            let mut cof_cover = Cover::empty();

            for (cube_index, cube) in cover.cubes().iter().enumerate() {
                if cube.contains(part) {
                    mapped_indices.push(cube_index);
                    cof_cover.push(cube.cofactored_on(variable, part));
                }
            }

            let cof_dont_care = Cover::new(
                dont_care
                    .cubes()
                    .iter()
                    .filter(|cube| cube.contains(part))
                    .map(|cube| cube.cofactored_on(variable, part)),
            );

            let irredundant = mark_irredundant(&cof_cover, &cof_dont_care, structure);
            for (cofactor_index, is_irredundant) in irredundant.iter().copied().enumerate() {
                if is_irredundant {
                    continue;
                }

                let cube_index = mapped_indices[cofactor_index];
                let is_output_variable = Some(variable_index) == structure.output_variable_index();
                let original_cube = &mut cover.cubes_mut()[cube_index];

                if is_output_variable || !original_cube.contains_all_parts(variable) {
                    original_cube.remove(part);
                }
            }
        }
    }

    let mut active = vec![true; cover.len()];
    for variable in structure
        .variables()
        .iter()
        .copied()
        .filter(|variable| variable.is_sparse())
    {
        for (index, cube) in cover.cubes().iter().enumerate() {
            if active[index] && cube.is_disjoint_from(variable) {
                active[index] = false;
            }
        }
    }

    cover.retain_active(&active);
    cover
}

pub fn mark_irredundant(cover: &Cover, dont_care: &Cover, structure: &CubeStructure) -> Vec<bool> {
    cover
        .cubes()
        .iter()
        .enumerate()
        .map(|(index, cube)| !cube_is_covered_by_others(cube, index, cover, dont_care, structure))
        .collect()
}

fn cube_is_covered_by_others(
    cube: &Cube,
    own_index: usize,
    cover: &Cover,
    dont_care: &Cover,
    structure: &CubeStructure,
) -> bool {
    let mut assignment = Vec::with_capacity(structure.variable_count());
    every_assignment_covered(
        cube,
        0,
        &mut assignment,
        |assignment| {
            dont_care
                .cubes()
                .iter()
                .any(|candidate| candidate.contains_assignment(assignment))
                || cover.cubes().iter().enumerate().any(|(index, candidate)| {
                    index != own_index && candidate.contains_assignment(assignment)
                })
        },
        structure,
    )
}

fn every_assignment_covered<P>(
    cube: &Cube,
    variable_index: usize,
    assignment: &mut Vec<usize>,
    mut predicate: P,
    structure: &CubeStructure,
) -> bool
where
    P: FnMut(&[usize]) -> bool,
{
    every_assignment_covered_inner(cube, variable_index, assignment, &mut predicate, structure)
}

fn every_assignment_covered_inner<P>(
    cube: &Cube,
    variable_index: usize,
    assignment: &mut Vec<usize>,
    predicate: &mut P,
    structure: &CubeStructure,
) -> bool
where
    P: FnMut(&[usize]) -> bool,
{
    if variable_index == structure.variable_count() {
        return predicate(assignment);
    }

    let variable = structure
        .variable(variable_index)
        .expect("valid variable index");
    for part in variable.parts() {
        if cube.contains(part) {
            assignment.push(part);
            if !every_assignment_covered_inner(
                cube,
                variable_index + 1,
                assignment,
                predicate,
                structure,
            ) {
                assignment.pop();
                return false;
            }
            assignment.pop();
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structure() -> CubeStructure {
        CubeStructure::new([
            Variable::new(0, 1, true),
            Variable::new(2, 3, false),
            Variable::new(4, 5, true),
        ])
    }

    fn cube(parts: &[usize]) -> Cube {
        Cube::from_parts(parts.iter().copied())
    }

    fn cover(cubes: &[&[usize]]) -> Cover {
        Cover::new(cubes.iter().map(|parts| cube(parts)))
    }

    #[test]
    fn cost_ignores_full_variables() {
        let structure = structure();
        let cover = cover(&[&[0, 1, 2, 4], &[1, 2, 3, 4, 5]]);

        assert_eq!(structure.cost(&cover), CoverCost { cubes: 2, total: 3 });
    }

    #[test]
    fn mark_irredundant_uses_dont_care_and_peer_cubes() {
        let structure = structure();
        let input = cover(&[&[0, 2, 4], &[0, 3, 4]]);
        let dont_care = cover(&[&[0, 2, 4]]);

        assert_eq!(
            mark_irredundant(&input, &dont_care, &structure),
            vec![false, true]
        );
    }

    #[test]
    fn mv_reduce_removes_redundant_sparse_parts() {
        let structure = structure();
        let input = cover(&[&[0, 2, 4], &[0, 3, 4], &[1, 2, 4]]);
        let dont_care = cover(&[&[0, 2, 4]]);

        let result = mv_reduce(input, &dont_care, &structure);

        assert_eq!(result, cover(&[&[0, 3, 4], &[1, 2, 4]]));
    }

    #[test]
    fn mv_reduce_does_not_reduce_full_non_output_variable() {
        let structure = CubeStructure::new([
            Variable::new(0, 1, true),
            Variable::new(2, 3, false),
            Variable::new(4, 5, false),
        ]);
        let input = cover(&[&[0, 1, 2, 4], &[0, 2, 4]]);
        let dont_care = cover(&[&[0, 1, 2, 4]]);

        let result = mv_reduce(input.clone(), &dont_care, &structure);

        assert!(result.cubes().contains(&cube(&[0, 1, 2, 4])));
    }

    #[test]
    fn mv_reduce_deletes_cubes_empty_in_sparse_variable() {
        let structure = structure();
        let input = cover(&[&[0, 2, 4], &[1, 2, 4]]);
        let dont_care = cover(&[&[0, 2, 4], &[1, 2, 4]]);

        let result = mv_reduce(input, &dont_care, &structure);

        assert!(result.is_empty());
    }

    #[test]
    fn make_sparse_alternates_reduce_and_expand_until_cost_stops() {
        let structure = structure();
        let input = cover(&[&[0, 2, 4], &[0, 3, 4], &[1, 2, 4]]);
        let dont_care = cover(&[&[0, 2, 4]]);
        let off_set = Cover::empty();
        let mut calls = 0;

        let result = make_sparse_with_expander(
            input,
            &dont_care,
            &off_set,
            &structure,
            SparseOptions::default(),
            |mut cover, _off_set| {
                calls += 1;
                cover.push(cube(&[1, 3, 5]));
                cover
            },
        );

        assert_eq!(calls, 1);
        assert_eq!(result, cover(&[&[0, 3, 4], &[1, 2, 4], &[1, 3, 5]]));
    }

    #[test]
    fn make_sparse_stops_after_reduction_when_total_is_unchanged() {
        let structure = structure();
        let input = cover(&[&[0, 2, 4]]);
        let dont_care = Cover::empty();
        let off_set = Cover::empty();
        let mut calls = 0;

        let result = make_sparse_with_expander(
            input.clone(),
            &dont_care,
            &off_set,
            &structure,
            SparseOptions::default(),
            |cover, _off_set| {
                calls += 1;
                cover
            },
        );

        assert_eq!(calls, 0);
        assert_eq!(result, input);
    }
}
