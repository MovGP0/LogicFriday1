//! Native Rust port of the two-cube kernel selection logic.
//!
//! The legacy routine builds every double-cube divisor for an SOP node, computes
//! the divisor weight used by fast extraction, lets a caller inspect each
//! candidate, and returns the candidate selected by that caller. This module
//! keeps that behavior in owned Rust data without exposing per-file C ABI shims.

#![allow(dead_code)]

use super::ddivisor::{
    DoubleCubeDivisorSet, DoubleCubeDivisorType, DoubleCubeExtractOptions,
    extract_double_cube_divisors,
};

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
    phase: usize,
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

    pub fn phase(&self) -> usize {
        self.phase
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

    let rows = node.cubes().iter().map(cube_to_columns).collect::<Vec<_>>();
    let mut divisor_set = DoubleCubeDivisorSet::new();
    extract_double_cube_divisors(
        &rows,
        0,
        0,
        &mut divisor_set,
        DoubleCubeExtractOptions::unrestricted(),
        || false,
    );

    divisor_set
        .divisors()
        .iter()
        .map(|divisor| {
            let cube1 = columns_to_cube(divisor.cube1());
            let cube2 = columns_to_cube(divisor.cube2());
            let occurrences = divisor
                .occurrence_indices()
                .iter()
                .map(|cell_index| {
                    let cell = &divisor_set.cells()[*cell_index];
                    DoubleCubeOccurrence {
                        first_cube: cell.cube1_row(),
                        second_cube: cell.cube2_row(),
                        base_length: cell.base_length(),
                        phase: cell.phase(),
                    }
                })
                .collect::<Vec<_>>();
            let weight =
                compute_divisor_weight(&rows, divisor.divisor_type(), &cube1, &cube2, &occurrences);

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
    rows: &[Vec<usize>],
    divisor_type: DoubleCubeDivisorType,
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
        + complementary_single_literal_weight(rows, divisor_type, cube1, cube2)
}

fn complementary_single_literal_weight(
    rows: &[Vec<usize>],
    divisor_type: DoubleCubeDivisorType,
    cube1: &Cube,
    cube2: &Cube,
) -> isize {
    if divisor_type != DoubleCubeDivisorType::D112
        || cube1.literal_count() != 1
        || cube2.literal_count() != 1
    {
        return 0;
    }

    let complement1 = literal_to_column(cube1.literals()[0].complement());
    let complement2 = literal_to_column(cube2.literals()[0].complement());

    rows.iter()
        .filter(|row| {
            row.binary_search(&complement1).is_ok() && row.binary_search(&complement2).is_ok()
        })
        .count() as isize
}

fn cube_to_columns(cube: &Cube) -> Vec<usize> {
    cube.literals()
        .iter()
        .copied()
        .map(literal_to_column)
        .collect()
}

fn columns_to_cube(columns: &[usize]) -> Cube {
    Cube::new(columns.iter().copied().map(column_to_literal))
}

fn literal_to_column(literal: CubeLiteral) -> usize {
    literal.variable() * 2 + usize::from(!literal.phase())
}

fn column_to_literal(column: usize) -> CubeLiteral {
    let variable = column / 2;
    if column % 2 == 0 {
        CubeLiteral::positive(variable)
    } else {
        CubeLiteral::negative(variable)
    }
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
    fn coalesces_d222_complement_phase_like_legacy_divisor_set() {
        let node = TwoCubeNode::sop([
            Cube::new([CubeLiteral::positive(0), CubeLiteral::positive(1)]),
            Cube::new([CubeLiteral::negative(0), CubeLiteral::negative(1)]),
            Cube::new([CubeLiteral::negative(0), CubeLiteral::positive(1)]),
            Cube::new([CubeLiteral::positive(0), CubeLiteral::negative(1)]),
        ]);

        let divisor = two_cube_kernels(&node)
            .into_iter()
            .find(|divisor| {
                divisor.cube1() == &Cube::new([CubeLiteral::positive(0), CubeLiteral::positive(1)])
                    && divisor.cube2()
                        == &Cube::new([CubeLiteral::negative(0), CubeLiteral::negative(1)])
            })
            .expect("expected D222 divisor");

        assert_eq!(divisor.occurrences().len(), 2);
        assert_eq!(divisor.occurrences()[0].phase(), 0);
        assert_eq!(divisor.occurrences()[1].phase(), 1);
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

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let source = include_str!("gen_2c_kernel.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("Logic", "Friday", "1-", "8j8")));
    }
}
