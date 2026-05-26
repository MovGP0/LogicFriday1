//! Native Rust port of `LogicSynthesis/sis/mincov/indep.c`.
//!
//! The SIS routine computes a greedy maximal independent set of sparse-matrix
//! rows. Two rows conflict when they share at least one column. The original C
//! implementation builds a row-intersection matrix, repeatedly chooses the row
//! with the fewest conflicts, adds it to the independent set, and removes all
//! rows that intersect it.

use crate::ports::sparse::matrix::SparseMatrix;

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndependentSetSolution {
    rows: BTreeSet<usize>,
    cost: i32,
}

impl IndependentSetSolution {
    pub fn new() -> Self {
        Self {
            rows: BTreeSet::new(),
            cost: 0,
        }
    }

    pub fn rows(&self) -> impl ExactSizeIterator<Item = usize> + '_ {
        self.rows.iter().copied()
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn contains_row(&self, row: usize) -> bool {
        self.rows.contains(&row)
    }

    pub fn cost(&self) -> i32 {
        self.cost
    }

    fn add_row(&mut self, row: usize, cost: i32) {
        if self.rows.insert(row) {
            self.cost += cost;
        }
    }
}

impl Default for IndependentSetSolution {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IndependentSetError {
    MissingWeight { col: usize },
    EmptyWeightedRow { row: usize },
}

impl fmt::Display for IndependentSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingWeight { col } => {
                write!(f, "missing mincov independent-set weight for column {col}")
            }
            Self::EmptyWeightedRow { row } => {
                write!(f, "cannot compute weighted cost for empty row {row}")
            }
        }
    }
}

impl Error for IndependentSetError {}

pub fn maximal_independent_set(
    matrix: &SparseMatrix,
    weights: Option<&[i32]>,
) -> Result<IndependentSetSolution, IndependentSetError> {
    let mut solution = IndependentSetSolution::new();
    let mut intersections = build_intersection_matrix(matrix);

    while intersections.row_count() > 0 {
        let best_row = intersections
            .rows()
            .min_by_key(|row| (row.len(), row.index()))
            .expect("intersection matrix row count is positive");
        let row_num = best_row.index();
        let selected_cost = selected_row_cost(matrix, row_num, weights)?;
        solution.add_row(row_num, selected_cost);

        let intersecting_rows = best_row.elements().to_vec();
        for row in intersecting_rows {
            intersections.delete_row(row);
            intersections.delete_col(row);
        }
    }

    Ok(solution)
}

pub fn build_intersection_matrix(matrix: &SparseMatrix) -> SparseMatrix {
    let mut intersections = SparseMatrix::new();

    for row in matrix.rows() {
        let mut reachable_rows = BTreeSet::new();
        for col in row.elements() {
            if let Some(column) = matrix.col(*col) {
                reachable_rows.extend(column.elements().iter().copied());
            }
        }

        for reachable_row in reachable_rows {
            intersections.insert(row.index(), reachable_row);
        }
    }

    intersections
}

pub fn verify_independent_rows(matrix: &SparseMatrix, rows: &[usize]) -> bool {
    for (index, row_num) in rows.iter().enumerate() {
        let Some(row) = matrix.row(*row_num) else {
            continue;
        };

        for other_row_num in &rows[index + 1..] {
            let Some(other_row) = matrix.row(*other_row_num) else {
                continue;
            };

            if row.elements().iter().any(|col| other_row.contains(*col)) {
                return false;
            }
        }
    }

    true
}

fn selected_row_cost(
    matrix: &SparseMatrix,
    row: usize,
    weights: Option<&[i32]>,
) -> Result<i32, IndependentSetError> {
    let Some(weights) = weights else {
        return Ok(1);
    };

    let sparse_row = matrix
        .row(row)
        .ok_or(IndependentSetError::EmptyWeightedRow { row })?;
    let mut columns = sparse_row.elements().iter().copied();
    let first_col = columns
        .next()
        .ok_or(IndependentSetError::EmptyWeightedRow { row })?;
    let mut least_weight = weight_at(weights, first_col)?;

    for col in columns {
        least_weight = least_weight.min(weight_at(weights, col)?);
    }

    Ok(least_weight)
}

fn weight_at(weights: &[i32], col: usize) -> Result<i32, IndependentSetError> {
    weights
        .get(col)
        .copied()
        .ok_or(IndependentSetError::MissingWeight { col })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix_from_rows(rows: &[&[usize]]) -> SparseMatrix {
        let mut matrix = SparseMatrix::new();
        for (row, cols) in rows.iter().enumerate() {
            for col in *cols {
                matrix.insert(row, *col);
            }
        }

        matrix
    }

    fn solution_rows(solution: &IndependentSetSolution) -> Vec<usize> {
        solution.rows().collect()
    }

    #[test]
    fn intersection_matrix_records_rows_that_share_columns() {
        let matrix = matrix_from_rows(&[&[0, 2], &[2, 4], &[5], &[0, 5]]);
        let intersections = build_intersection_matrix(&matrix);

        assert_eq!(intersections.row(0).unwrap().elements(), &[0, 1, 3]);
        assert_eq!(intersections.row(1).unwrap().elements(), &[0, 1]);
        assert_eq!(intersections.row(2).unwrap().elements(), &[2, 3]);
        assert_eq!(intersections.row(3).unwrap().elements(), &[0, 2, 3]);
    }

    #[test]
    fn maximal_independent_set_chooses_minimum_intersection_row_first() {
        let matrix = matrix_from_rows(&[&[0, 1], &[1, 2], &[3], &[0, 3]]);
        let solution = maximal_independent_set(&matrix, None).unwrap();
        let rows = solution_rows(&solution);

        assert_eq!(rows, vec![1, 2]);
        assert_eq!(solution.row_count(), 2);
        assert_eq!(solution.cost(), 2);
        assert!(verify_independent_rows(&matrix, &rows));
    }

    #[test]
    fn maximal_independent_set_uses_least_weight_in_selected_row() {
        let matrix = matrix_from_rows(&[&[0, 4], &[1], &[4], &[2, 3]]);
        let weights = [9, 5, 7, 3, 2];
        let solution = maximal_independent_set(&matrix, Some(&weights)).unwrap();

        assert_eq!(solution_rows(&solution), vec![0, 1, 3]);
        assert_eq!(solution.cost(), 10);
    }

    #[test]
    fn tie_keeps_first_sorted_row_like_c_list_walk() {
        let matrix = matrix_from_rows(&[&[0], &[1], &[2]]);
        let solution = maximal_independent_set(&matrix, None).unwrap();

        assert_eq!(solution_rows(&solution), vec![0, 1, 2]);
    }

    #[test]
    fn verifies_independent_and_conflicting_row_sets() {
        let matrix = matrix_from_rows(&[&[0], &[1], &[0, 2], &[3]]);

        assert!(verify_independent_rows(&matrix, &[0, 1, 3]));
        assert!(!verify_independent_rows(&matrix, &[0, 2]));
    }

    #[test]
    fn reports_missing_weight_instead_of_indexing_past_slice() {
        let matrix = matrix_from_rows(&[&[3]]);

        assert_eq!(
            maximal_independent_set(&matrix, Some(&[1, 2])),
            Err(IndependentSetError::MissingWeight { col: 3 })
        );
    }

    #[test]
    fn reports_empty_weighted_row_when_intersection_row_has_no_source_row() {
        let matrix = SparseMatrix::new();

        assert_eq!(
            super::selected_row_cost(&matrix, 9, Some(&[1])),
            Err(IndependentSetError::EmptyWeightedRow { row: 9 })
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present() {
        let source = include_str!("indep.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("Logic", "Friday1-")));
        assert!(!source.contains(concat!("bd ", "dep")));
    }
}
