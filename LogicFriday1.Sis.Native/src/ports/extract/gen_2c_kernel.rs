//! Native Rust port of the two-cube kernel selection logic.
//!
//! The legacy routine builds every double-cube divisor for an SOP node, computes
//! the divisor weight used by fast extraction, lets a caller inspect each
//! candidate, and returns the candidate selected by that caller. This module
//! keeps that behavior in owned Rust data without exposing per-file C ABI shims.

#![allow(dead_code)]

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TwoCubeNodeFunction {
    PrimaryInput,
    PrimaryOutput,
    Zero,
    One,
    Inverter,
    Buffer,
    And,
    Or,
    Complex,
}

impl TwoCubeNodeFunction {
    fn can_have_two_cube_kernels(self) -> bool {
        matches!(self, Self::And | Self::Or | Self::Complex)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CubeLiteral {
    variable: usize,
    phase: bool,
}

impl CubeLiteral {
    pub const fn positive(variable: usize) -> Self {
        Self {
            variable,
            phase: true,
        }
    }

    pub const fn negative(variable: usize) -> Self {
        Self {
            variable,
            phase: false,
        }
    }

    pub const fn variable(self) -> usize {
        self.variable
    }

    pub const fn phase(self) -> bool {
        self.phase
    }

    pub const fn complement(self) -> Self {
        Self {
            variable: self.variable,
            phase: !self.phase,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Cube {
    literals: Vec<CubeLiteral>,
}

impl Cube {
    pub fn new(literals: impl IntoIterator<Item = CubeLiteral>) -> Self {
        let mut literals = literals.into_iter().collect::<Vec<_>>();
        literals.sort_unstable();
        literals.dedup();

        Self { literals }
    }

    pub fn literals(&self) -> &[CubeLiteral] {
        &self.literals
    }

    pub fn literal_count(&self) -> usize {
        self.literals.len()
    }

    pub fn contains(&self, literal: CubeLiteral) -> bool {
        self.literals.binary_search(&literal).is_ok()
    }

    fn common_literals(&self, other: &Self) -> Vec<CubeLiteral> {
        self.literals
            .iter()
            .copied()
            .filter(|literal| other.contains(*literal))
            .collect()
    }

    fn without_literals(&self, literals_to_remove: &[CubeLiteral]) -> Self {
        Self::new(
            self.literals
                .iter()
                .copied()
                .filter(|literal| literals_to_remove.binary_search(literal).is_err()),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TwoCubeNode {
    function: TwoCubeNodeFunction,
    cubes: Vec<Cube>,
}

impl TwoCubeNode {
    pub fn new(function: TwoCubeNodeFunction, cubes: impl IntoIterator<Item = Cube>) -> Self {
        Self {
            function,
            cubes: cubes.into_iter().collect(),
        }
    }

    pub fn sop(cubes: impl IntoIterator<Item = Cube>) -> Self {
        Self::new(TwoCubeNodeFunction::Complex, cubes)
    }

    pub fn function(&self) -> TwoCubeNodeFunction {
        self.function
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoubleCubeOccurrence {
    first_cube: usize,
    second_cube: usize,
    base_length: usize,
}

impl DoubleCubeOccurrence {
    pub fn first_cube(&self) -> usize {
        self.first_cube
    }

    pub fn second_cube(&self) -> usize {
        self.second_cube
    }

    pub fn base_length(&self) -> usize {
        self.base_length
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoubleCubeDivisor {
    cube1: Cube,
    cube2: Cube,
    occurrences: Vec<DoubleCubeOccurrence>,
    weight: isize,
}

impl DoubleCubeDivisor {
    pub fn cube1(&self) -> &Cube {
        &self.cube1
    }

    pub fn cube2(&self) -> &Cube {
        &self.cube2
    }

    pub fn occurrences(&self) -> &[DoubleCubeOccurrence] {
        &self.occurrences
    }

    pub fn weight(&self) -> isize {
        self.weight
    }

    pub fn to_node(&self) -> TwoCubeNode {
        TwoCubeNode::sop([self.cube1.clone(), self.cube2.clone()])
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelVisit {
    Continue,
    Stop,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KernelSelection {
    selected: Option<usize>,
}

impl KernelSelection {
    pub fn select_current(&mut self, index: usize) {
        self.selected = Some(index);
    }

    pub fn clear(&mut self) {
        self.selected = None;
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }
}

pub fn two_cube_kernels(node: &TwoCubeNode) -> Vec<DoubleCubeDivisor> {
    if !node.function.can_have_two_cube_kernels() {
        return Vec::new();
    }

    let mut divisors = BTreeMap::<(Cube, Cube), Vec<DoubleCubeOccurrence>>::new();
    for first_index in 0..node.cubes.len() {
        for second_index in (first_index + 1)..node.cubes.len() {
            let first = &node.cubes[first_index];
            let second = &node.cubes[second_index];
            let base = first.common_literals(second);
            let mut residual1 = first.without_literals(&base);
            let mut residual2 = second.without_literals(&base);

            if residual2.literal_count() < residual1.literal_count()
                || (residual1.literal_count() == residual2.literal_count() && residual2 < residual1)
            {
                std::mem::swap(&mut residual1, &mut residual2);
            }

            divisors
                .entry((residual1, residual2))
                .or_default()
                .push(DoubleCubeOccurrence {
                    first_cube: first_index,
                    second_cube: second_index,
                    base_length: base.len(),
                });
        }
    }

    divisors
        .into_iter()
        .map(|((cube1, cube2), occurrences)| {
            let weight = compute_divisor_weight(node, &cube1, &cube2, &occurrences);
            DoubleCubeDivisor {
                cube1,
                cube2,
                occurrences,
                weight,
            }
        })
        .collect()
}

pub fn two_cube_kernel_best_with<State>(
    node: &TwoCubeNode,
    state: &mut State,
    mut evaluate: impl FnMut(
        &DoubleCubeDivisor,
        &TwoCubeNode,
        &mut KernelSelection,
        &mut State,
    ) -> KernelVisit,
) -> Option<TwoCubeNode> {
    let divisors = two_cube_kernels(node);
    let mut selection = KernelSelection::default();

    for (index, divisor) in divisors.iter().enumerate() {
        let candidate = divisor.to_node();
        if evaluate(divisor, &candidate, &mut selection, state) == KernelVisit::Stop {
            selection.clear();
            break;
        }

        if selection.selected() == Some(usize::MAX) {
            selection.select_current(index);
        }
    }

    selection
        .selected()
        .and_then(|index| divisors.get(index))
        .map(DoubleCubeDivisor::to_node)
}

pub fn best_two_cube_kernel_by_weight(node: &TwoCubeNode) -> Option<TwoCubeNode> {
    let mut best = None::<(usize, isize)>;
    let mut index = 0;
    let mut state = ();

    two_cube_kernel_best_with(node, &mut state, |divisor, _, selection, _| {
        if best.is_none_or(|(_, weight)| divisor.weight() > weight) {
            best = Some((index, divisor.weight()));
            selection.select_current(index);
        }
        index += 1;

        KernelVisit::Continue
    })
}

fn compute_divisor_weight(
    node: &TwoCubeNode,
    cube1: &Cube,
    cube2: &Cube,
    occurrences: &[DoubleCubeOccurrence],
) -> isize {
    let occurrence_count = occurrences.len() as isize;
    let common_base_saving = occurrences
        .iter()
        .map(|occurrence| occurrence.base_length as isize)
        .sum::<isize>();
    let extracted_literal_saving =
        (occurrence_count - 1) * (cube1.literal_count() + cube2.literal_count()) as isize;

    extracted_literal_saving + common_base_saving - occurrence_count
        + complementary_single_literal_weight(node, cube1, cube2)
}

fn complementary_single_literal_weight(node: &TwoCubeNode, cube1: &Cube, cube2: &Cube) -> isize {
    if cube1.literal_count() != 1 || cube2.literal_count() != 1 {
        return 0;
    }

    let complement1 = cube1.literals()[0].complement();
    let complement2 = cube2.literals()[0].complement();

    node.cubes()
        .iter()
        .filter(|cube| cube.contains(complement1) && cube.contains(complement2))
        .count() as isize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_no_kernel_for_trivial_node_functions() {
        let node = TwoCubeNode::new(
            TwoCubeNodeFunction::Buffer,
            [Cube::new([CubeLiteral::positive(0)])],
        );

        assert!(two_cube_kernels(&node).is_empty());
        assert_eq!(best_two_cube_kernel_by_weight(&node), None);
    }

    #[test]
    fn removes_common_base_literals_from_generated_divisor() {
        let node = TwoCubeNode::sop([
            Cube::new([CubeLiteral::positive(0), CubeLiteral::positive(1)]),
            Cube::new([CubeLiteral::positive(0), CubeLiteral::negative(2)]),
        ]);

        let kernels = two_cube_kernels(&node);

        assert_eq!(kernels.len(), 1);
        assert_eq!(kernels[0].cube1(), &Cube::new([CubeLiteral::positive(1)]));
        assert_eq!(kernels[0].cube2(), &Cube::new([CubeLiteral::negative(2)]));
        assert_eq!(kernels[0].occurrences()[0].base_length(), 1);
    }

    #[test]
    fn combines_equal_divisors_and_computes_fast_extract_weight() {
        let node = TwoCubeNode::sop([
            Cube::new([CubeLiteral::positive(0), CubeLiteral::positive(1)]),
            Cube::new([CubeLiteral::positive(0), CubeLiteral::positive(2)]),
            Cube::new([CubeLiteral::negative(3), CubeLiteral::positive(1)]),
            Cube::new([CubeLiteral::negative(3), CubeLiteral::positive(2)]),
        ]);

        let divisor = two_cube_kernels(&node)
            .into_iter()
            .find(|divisor| {
                divisor.cube1() == &Cube::new([CubeLiteral::positive(1)])
                    && divisor.cube2() == &Cube::new([CubeLiteral::positive(2)])
            })
            .expect("expected shared divisor");

        assert_eq!(divisor.occurrences().len(), 2);
        assert_eq!(divisor.weight(), 2);
    }

    #[test]
    fn includes_single_literal_complement_weight() {
        let node = TwoCubeNode::sop([
            Cube::new([CubeLiteral::positive(0)]),
            Cube::new([CubeLiteral::positive(1)]),
            Cube::new([CubeLiteral::negative(0), CubeLiteral::negative(1)]),
        ]);

        let divisor = two_cube_kernels(&node)
            .into_iter()
            .find(|divisor| {
                divisor.cube1() == &Cube::new([CubeLiteral::positive(0)])
                    && divisor.cube2() == &Cube::new([CubeLiteral::positive(1)])
            })
            .expect("expected single literal divisor");

        assert_eq!(divisor.weight(), 0);
    }

    #[test]
    fn callback_selects_best_candidate_by_handle_like_selection() {
        let node = TwoCubeNode::sop([
            Cube::new([CubeLiteral::positive(0), CubeLiteral::positive(1)]),
            Cube::new([CubeLiteral::positive(0), CubeLiteral::positive(2)]),
            Cube::new([CubeLiteral::negative(3), CubeLiteral::positive(1)]),
            Cube::new([CubeLiteral::negative(3), CubeLiteral::positive(2)]),
            Cube::new([CubeLiteral::negative(4), CubeLiteral::positive(1)]),
            Cube::new([CubeLiteral::negative(4), CubeLiteral::positive(2)]),
        ]);
        let mut best_weight = isize::MIN;
        let mut index = 0;

        let best =
            two_cube_kernel_best_with(&node, &mut best_weight, |divisor, _, selection, state| {
                if divisor.weight() > *state {
                    *state = divisor.weight();
                    selection.select_current(index);
                }
                index += 1;

                KernelVisit::Continue
            });

        assert_eq!(
            best,
            Some(TwoCubeNode::sop([
                Cube::new([CubeLiteral::positive(1)]),
                Cube::new([CubeLiteral::positive(2)]),
            ]))
        );
    }

    #[test]
    fn callback_stop_clears_selection_like_legacy_break_path() {
        let node = TwoCubeNode::sop([
            Cube::new([CubeLiteral::positive(0)]),
            Cube::new([CubeLiteral::positive(1)]),
        ]);
        let mut state = ();

        let best = two_cube_kernel_best_with(&node, &mut state, |_, _, selection, _| {
            selection.select_current(0);
            KernelVisit::Stop
        });

        assert_eq!(best, None);
    }
}
