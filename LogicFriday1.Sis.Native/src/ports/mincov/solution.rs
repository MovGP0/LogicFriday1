//! Native Rust port of `LogicSynthesis/sis/mincov/solution.c`.
//!
//! The C module owns the selected cover columns and their accumulated cost.
//! This port keeps that behavior in a small value type backed by the native
//! sparse matrix port. Legacy allocator/free routines are represented by Rust
//! construction, cloning, and ownership.

use crate::ports::sparse::matrix::SparseMatrix;

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MincovSolution {
    selected_columns: BTreeSet<usize>,
    cost: i32,
}

impl MincovSolution {
    pub fn new() -> Self {
        Self {
            selected_columns: BTreeSet::new(),
            cost: 0,
        }
    }

    pub fn with_selected_columns<I>(columns: I, weights: Option<&[i32]>) -> MincovResult<Self>
    where
        I: IntoIterator<Item = usize>,
    {
        let mut solution = Self::new();
        for column in columns {
            solution.add(weights, column)?;
        }

        Ok(solution)
    }

    pub fn selected_columns(&self) -> &BTreeSet<usize> {
        &self.selected_columns
    }

    pub fn cost(&self) -> i32 {
        self.cost
    }

    pub fn contains(&self, column: usize) -> bool {
        self.selected_columns.contains(&column)
    }

    pub fn add(&mut self, weights: Option<&[i32]>, column: usize) -> MincovResult<bool> {
        let column_weight = weight(weights, column)?;
        let inserted = self.selected_columns.insert(column);
        self.cost = self
            .cost
            .checked_add(column_weight)
            .ok_or(MincovSolutionError::CostOverflow)?;

        Ok(inserted)
    }

    pub fn accept(
        &mut self,
        matrix: &mut SparseMatrix,
        weights: Option<&[i32]>,
        column: usize,
    ) -> MincovResult<()> {
        self.add(weights, column)?;

        let covered_rows = matrix
            .col(column)
            .map(|col| col.elements().to_vec())
            .unwrap_or_default();

        for row in covered_rows {
            matrix.delete_row(row);
        }

        Ok(())
    }

    pub fn reject(&mut self, matrix: &mut SparseMatrix, column: usize) {
        matrix.delete_col(column);
    }

    pub fn verify_cover(&self, matrix: &SparseMatrix) -> bool {
        matrix
            .rows()
            .all(|row| row.elements().iter().any(|column| self.contains(*column)))
    }

    pub fn into_selected_columns(self) -> BTreeSet<usize> {
        self.selected_columns
    }
}

impl Default for MincovSolution {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MincovSolutionError {
    MissingWeight { column: usize },
    CostOverflow,
}

impl fmt::Display for MincovSolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingWeight { column } => {
                write!(f, "missing mincov weight for column {column}")
            }
            Self::CostOverflow => write!(f, "mincov solution cost overflowed"),
        }
    }
}

impl Error for MincovSolutionError {}

pub type MincovResult<T> = Result<T, MincovSolutionError>;

pub fn choose_best(
    best1: Option<MincovSolution>,
    best2: Option<MincovSolution>,
) -> Option<MincovSolution> {
    match (best1, best2) {
        (Some(left), Some(right)) if left.cost <= right.cost => Some(left),
        (Some(_), Some(right)) => Some(right),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn weight(weights: Option<&[i32]>, column: usize) -> MincovResult<i32> {
    match weights {
        Some(weights) => weights
            .get(column)
            .copied()
            .ok_or(MincovSolutionError::MissingWeight { column }),
        None => Ok(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix_from_pairs(pairs: &[(usize, usize)]) -> SparseMatrix {
        let mut matrix = SparseMatrix::new();
        for (row, column) in pairs {
            matrix.insert(*row, *column);
        }

        matrix
    }

    fn set(values: &[usize]) -> BTreeSet<usize> {
        values.iter().copied().collect()
    }

    #[test]
    fn new_solution_matches_c_allocation_defaults() {
        let solution = MincovSolution::new();

        assert_eq!(solution.cost(), 0);
        assert!(solution.selected_columns().is_empty());
    }

    #[test]
    fn add_inserts_column_and_accumulates_weight_on_each_call() {
        let mut solution = MincovSolution::new();
        let weights = [4, 9, 2];

        assert_eq!(solution.add(Some(&weights), 2), Ok(true));
        assert_eq!(solution.add(Some(&weights), 2), Ok(false));
        assert_eq!(solution.add(Some(&weights), 0), Ok(true));

        assert_eq!(solution.selected_columns(), &set(&[0, 2]));
        assert_eq!(solution.cost(), 8);
    }

    #[test]
    fn add_uses_unit_weight_when_weight_table_is_absent() {
        let mut solution = MincovSolution::new();

        assert_eq!(solution.add(None, 7), Ok(true));
        assert_eq!(solution.cost(), 1);
    }

    #[test]
    fn add_reports_missing_weight_instead_of_indexing_past_slice() {
        let mut solution = MincovSolution::new();

        assert_eq!(
            solution.add(Some(&[1, 2]), 4),
            Err(MincovSolutionError::MissingWeight { column: 4 })
        );
        assert!(solution.selected_columns().is_empty());
        assert_eq!(solution.cost(), 0);
    }

    #[test]
    fn accept_selects_column_and_deletes_covered_rows() {
        let mut matrix = matrix_from_pairs(&[(0, 1), (0, 2), (1, 2), (2, 3)]);
        let mut solution = MincovSolution::new();

        solution.accept(&mut matrix, None, 2).unwrap();

        assert_eq!(solution.selected_columns(), &set(&[2]));
        assert_eq!(solution.cost(), 1);
        assert!(matrix.row(0).is_none());
        assert!(matrix.row(1).is_none());
        assert_eq!(matrix.row(2).unwrap().elements(), &[3]);
        assert!(matrix.col(1).is_none());
        assert!(matrix.col(2).is_none());
    }

    #[test]
    fn reject_deletes_column_without_changing_selected_set() {
        let mut matrix = matrix_from_pairs(&[(0, 1), (0, 2), (1, 2), (2, 3)]);
        let mut solution = MincovSolution::new();

        solution.reject(&mut matrix, 2);

        assert!(solution.selected_columns().is_empty());
        assert_eq!(solution.cost(), 0);
        assert_eq!(matrix.row(0).unwrap().elements(), &[1]);
        assert!(matrix.row(1).is_none());
        assert!(matrix.col(2).is_none());
    }

    #[test]
    fn choose_best_prefers_lower_cost_and_keeps_left_on_tie() {
        let left = MincovSolution::with_selected_columns([1, 3], Some(&[0, 4, 0, 2])).unwrap();
        let right = MincovSolution::with_selected_columns([2], Some(&[0, 0, 9])).unwrap();
        let tie = MincovSolution::with_selected_columns([4], Some(&[0, 0, 0, 0, 6])).unwrap();

        assert_eq!(
            choose_best(Some(left.clone()), Some(right))
                .unwrap()
                .selected_columns(),
            &set(&[1, 3])
        );
        assert_eq!(
            choose_best(Some(left.clone()), Some(tie))
                .unwrap()
                .selected_columns(),
            &set(&[1, 3])
        );
        assert_eq!(choose_best(Some(left.clone()), None), Some(left));
        assert_eq!(choose_best(None, None), None);
    }

    #[test]
    fn verify_cover_checks_every_original_matrix_row() {
        let matrix = matrix_from_pairs(&[(0, 1), (0, 2), (1, 2), (2, 3)]);
        let partial = MincovSolution::with_selected_columns([2], None).unwrap();
        let complete = MincovSolution::with_selected_columns([2, 3], None).unwrap();

        assert!(!partial.verify_cover(&matrix));
        assert!(complete.verify_cover(&matrix));
    }

    #[test]
    fn reports_cost_overflow_without_wrapping() {
        let mut solution = MincovSolution::new();
        let weights = [i32::MAX, 1];

        assert_eq!(solution.add(Some(&weights), 0), Ok(true));
        assert_eq!(
            solution.add(Some(&weights), 1),
            Err(MincovSolutionError::CostOverflow)
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present() {
        let source = include_str!("solution.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("Logic", "Friday1-")));
        assert!(!source.contains(concat!("bd ", "dep")));
    }
}
