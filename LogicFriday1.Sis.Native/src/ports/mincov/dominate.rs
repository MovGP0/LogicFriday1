//! Native Rust model for `LogicSynthesis/sis/mincov/dominate.c`.
//!
//! The legacy routines reduce a sparse covering matrix by removing dominated
//! rows and columns. This port keeps the same containment rules and stable
//! tie-breaks while using owned Rust sparse sets.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RowId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ColId(pub usize);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SparseMatrix {
    rows: BTreeMap<RowId, BTreeSet<ColId>>,
    cols: BTreeMap<ColId, BTreeSet<RowId>>,
}

impl SparseMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, row: RowId, col: ColId) -> bool {
        let row_inserted = self.rows.entry(row).or_default().insert(col);
        let col_inserted = self.cols.entry(col).or_default().insert(row);
        debug_assert_eq!(row_inserted, col_inserted);
        row_inserted
    }

    pub fn contains(&self, row: RowId, col: ColId) -> bool {
        self.rows.get(&row).is_some_and(|cols| cols.contains(&col))
    }

    pub fn row(&self, row: RowId) -> Option<&BTreeSet<ColId>> {
        self.rows.get(&row)
    }

    pub fn col(&self, col: ColId) -> Option<&BTreeSet<RowId>> {
        self.cols.get(&col)
    }

    pub fn rows(&self) -> &BTreeMap<RowId, BTreeSet<ColId>> {
        &self.rows
    }

    pub fn cols(&self) -> &BTreeMap<ColId, BTreeSet<RowId>> {
        &self.cols
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize {
        self.cols.len()
    }

    pub fn row_length(&self, row: RowId) -> Option<usize> {
        self.row(row).map(BTreeSet::len)
    }

    pub fn col_length(&self, col: ColId) -> Option<usize> {
        self.col(col).map(BTreeSet::len)
    }

    pub fn delrow(&mut self, row: RowId) -> bool {
        let Some(cols) = self.rows.remove(&row) else {
            return false;
        };

        for col in cols {
            if let Some(rows) = self.cols.get_mut(&col) {
                rows.remove(&row);
                if rows.is_empty() {
                    self.cols.remove(&col);
                }
            }
        }
        true
    }

    pub fn delcol(&mut self, col: ColId) -> bool {
        let Some(rows) = self.cols.remove(&col) else {
            return false;
        };

        for row in rows {
            if let Some(cols) = self.rows.get_mut(&row) {
                cols.remove(&col);
                if cols.is_empty() {
                    self.rows.remove(&row);
                }
            }
        }
        true
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DominanceError {
    MissingWeight { col: ColId, weights: usize },
}

impl fmt::Display for DominanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingWeight { col, weights } => write!(
                f,
                "missing mincov dominance weight for column {} in {} supplied weights",
                col.0, weights
            ),
        }
    }
}

impl Error for DominanceError {}

pub type DominanceResult<T> = Result<T, DominanceError>;

pub fn sm_row_dominance(matrix: &mut SparseMatrix) -> usize {
    let initial_row_count = matrix.row_count();
    let row_order: Vec<_> = matrix.rows.keys().copied().collect();

    for row in row_order {
        let Some(row_cols) = matrix.row(row).cloned() else {
            continue;
        };
        let Some(least_col) = least_col_by_length(matrix, &row_cols) else {
            continue;
        };

        let candidate_rows: Vec<_> = matrix
            .col(least_col)
            .map(|rows| rows.iter().copied().collect())
            .unwrap_or_default();

        for candidate in candidate_rows {
            let Some(candidate_cols) = matrix.row(candidate) else {
                continue;
            };

            if is_row_dominance_candidate(row, row_cols.len(), candidate, candidate_cols.len())
                && row_contains(&row_cols, candidate_cols)
            {
                matrix.delrow(candidate);
            }
        }
    }

    initial_row_count - matrix.row_count()
}

pub fn sm_col_dominance(
    matrix: &mut SparseMatrix,
    weights: Option<&[i32]>,
) -> DominanceResult<usize> {
    let initial_col_count = matrix.col_count();
    let col_order: Vec<_> = matrix.cols.keys().copied().collect();

    for col in col_order {
        let Some(col_rows) = matrix.col(col).cloned() else {
            continue;
        };
        let Some(least_row) = least_row_by_length(matrix, &col_rows) else {
            continue;
        };

        let candidate_cols: Vec<_> = matrix
            .row(least_row)
            .map(|cols| cols.iter().copied().collect())
            .unwrap_or_default();

        for candidate in candidate_cols {
            let Some(candidate_rows) = matrix.col(candidate) else {
                continue;
            };
            if is_more_expensive(candidate, col, weights)? {
                continue;
            }
            if is_col_dominance_candidate(col, col_rows.len(), candidate, candidate_rows.len())
                && col_contains(&col_rows, candidate_rows)
            {
                matrix.delcol(col);
                break;
            }
        }
    }

    Ok(initial_col_count - matrix.col_count())
}

pub fn row_contains(row: &BTreeSet<ColId>, candidate: &BTreeSet<ColId>) -> bool {
    row.is_subset(candidate)
}

pub fn col_contains(col: &BTreeSet<RowId>, candidate: &BTreeSet<RowId>) -> bool {
    col.is_subset(candidate)
}

fn least_col_by_length(matrix: &SparseMatrix, cols: &BTreeSet<ColId>) -> Option<ColId> {
    cols.iter()
        .copied()
        .min_by_key(|col| (matrix.col_length(*col).unwrap_or(0), *col))
}

fn least_row_by_length(matrix: &SparseMatrix, rows: &BTreeSet<RowId>) -> Option<RowId> {
    rows.iter()
        .copied()
        .min_by_key(|row| (matrix.row_length(*row).unwrap_or(0), *row))
}

fn is_row_dominance_candidate(
    row: RowId,
    row_length: usize,
    candidate: RowId,
    candidate_length: usize,
) -> bool {
    candidate_length > row_length || (candidate_length == row_length && candidate > row)
}

fn is_col_dominance_candidate(
    col: ColId,
    col_length: usize,
    candidate: ColId,
    candidate_length: usize,
) -> bool {
    candidate_length > col_length || (candidate_length == col_length && candidate > col)
}

fn is_more_expensive(
    candidate: ColId,
    current: ColId,
    weights: Option<&[i32]>,
) -> DominanceResult<bool> {
    let Some(weights) = weights else {
        return Ok(false);
    };

    Ok(weight(weights, candidate)? > weight(weights, current)?)
}

fn weight(weights: &[i32], col: ColId) -> DominanceResult<i32> {
    weights
        .get(col.0)
        .copied()
        .ok_or(DominanceError::MissingWeight {
            col,
            weights: weights.len(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(entries: &[(usize, usize)]) -> SparseMatrix {
        let mut matrix = SparseMatrix::new();
        for &(row, col) in entries {
            matrix.insert(RowId(row), ColId(col));
        }
        matrix
    }

    fn row_values(matrix: &SparseMatrix, row: usize) -> Vec<usize> {
        matrix
            .row(RowId(row))
            .map(|cols| cols.iter().map(|col| col.0).collect())
            .unwrap_or_default()
    }

    fn col_values(matrix: &SparseMatrix, col: usize) -> Vec<usize> {
        matrix
            .col(ColId(col))
            .map(|rows| rows.iter().map(|row| row.0).collect())
            .unwrap_or_default()
    }

    #[test]
    fn row_dominance_removes_superset_rows() {
        let mut matrix = matrix(&[(0, 1), (0, 2), (1, 1), (1, 2), (1, 3), (2, 3)]);

        let removed = sm_row_dominance(&mut matrix);

        assert_eq!(removed, 1);
        assert_eq!(row_values(&matrix, 0), vec![1, 2]);
        assert_eq!(row_values(&matrix, 1), Vec::<usize>::new());
        assert_eq!(row_values(&matrix, 2), vec![3]);
    }

    #[test]
    fn row_dominance_uses_row_number_tie_break_for_equal_rows() {
        let mut matrix = matrix(&[(1, 4), (1, 5), (2, 4), (2, 5), (3, 6)]);

        let removed = sm_row_dominance(&mut matrix);

        assert_eq!(removed, 1);
        assert_eq!(row_values(&matrix, 1), vec![4, 5]);
        assert_eq!(row_values(&matrix, 2), Vec::<usize>::new());
        assert_eq!(row_values(&matrix, 3), vec![6]);
    }

    #[test]
    fn row_dominance_limits_candidates_to_shortest_incident_column() {
        let mut matrix = matrix(&[
            (0, 1),
            (0, 2),
            (1, 1),
            (1, 2),
            (1, 3),
            (2, 1),
            (2, 2),
            (2, 4),
            (9, 8),
        ]);

        let removed = sm_row_dominance(&mut matrix);

        assert_eq!(removed, 2);
        assert_eq!(row_values(&matrix, 1), Vec::<usize>::new());
        assert_eq!(row_values(&matrix, 2), Vec::<usize>::new());
        assert_eq!(row_values(&matrix, 9), vec![8]);
    }

    #[test]
    fn column_dominance_removes_columns_with_covering_cheaper_supersets() {
        let mut matrix = matrix(&[(0, 0), (1, 0), (0, 1), (1, 1), (2, 1), (3, 2)]);

        let removed = sm_col_dominance(&mut matrix, None).unwrap();

        assert_eq!(removed, 1);
        assert_eq!(col_values(&matrix, 0), Vec::<usize>::new());
        assert_eq!(col_values(&matrix, 1), vec![0, 1, 2]);
        assert_eq!(col_values(&matrix, 2), vec![3]);
    }

    #[test]
    fn column_dominance_preserves_columns_when_superset_is_more_expensive() {
        let mut matrix = matrix(&[(0, 0), (1, 0), (0, 1), (1, 1), (2, 1)]);
        let weights = [3, 5];

        let removed = sm_col_dominance(&mut matrix, Some(&weights)).unwrap();

        assert_eq!(removed, 0);
        assert_eq!(col_values(&matrix, 0), vec![0, 1]);
        assert_eq!(col_values(&matrix, 1), vec![0, 1, 2]);
    }

    #[test]
    fn column_dominance_uses_column_number_tie_break_for_equal_columns() {
        let mut matrix = matrix(&[(0, 3), (1, 3), (0, 4), (1, 4)]);
        let weights = [1, 1, 1, 7, 7];

        let removed = sm_col_dominance(&mut matrix, Some(&weights)).unwrap();

        assert_eq!(removed, 1);
        assert_eq!(col_values(&matrix, 3), Vec::<usize>::new());
        assert_eq!(col_values(&matrix, 4), vec![0, 1]);
    }

    #[test]
    fn column_dominance_reports_missing_weight() {
        let mut matrix = matrix(&[(0, 0), (0, 2)]);

        let error = sm_col_dominance(&mut matrix, Some(&[1])).unwrap_err();

        assert_eq!(
            error,
            DominanceError::MissingWeight {
                col: ColId(2),
                weights: 1
            }
        );
    }

    #[test]
    fn deleting_rows_and_columns_keeps_indexes_consistent() {
        let mut matrix = matrix(&[(0, 0), (0, 1), (1, 1), (2, 2)]);

        assert!(matrix.delrow(RowId(0)));
        assert!(!matrix.contains(RowId(0), ColId(0)));
        assert_eq!(col_values(&matrix, 1), vec![1]);

        assert!(matrix.delcol(ColId(1)));
        assert_eq!(row_values(&matrix, 1), Vec::<usize>::new());
        assert_eq!(row_values(&matrix, 2), vec![2]);
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present() {
        let source = include_str!("dominate.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
    }
}
