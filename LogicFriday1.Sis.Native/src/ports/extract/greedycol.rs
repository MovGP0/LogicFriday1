//! Native Rust implementation of SIS greedy column rectangle extraction.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GreedyColumnMatrix {
    rows: BTreeMap<usize, BTreeMap<usize, i32>>,
    cols: BTreeMap<usize, BTreeMap<usize, i32>>,
}

impl GreedyColumnMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, row: usize, col: usize, value: i32) -> Option<i32> {
        let old = self.rows.entry(row).or_default().insert(col, value);
        self.cols.entry(col).or_default().insert(row, value);
        old
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize {
        self.cols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cols.is_empty()
    }

    pub fn value(&self, row: usize, col: usize) -> Option<i32> {
        self.rows.get(&row).and_then(|cols| cols.get(&col)).copied()
    }

    pub fn row(&self, row: usize) -> Option<&BTreeMap<usize, i32>> {
        self.rows.get(&row)
    }

    pub fn col(&self, col: usize) -> Option<&BTreeMap<usize, i32>> {
        self.cols.get(&col)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GreedyRectangle {
    pub cols: BTreeSet<usize>,
    pub rows: BTreeSet<usize>,
    pub value: i32,
}

impl GreedyRectangle {
    pub fn is_empty(&self) -> bool {
        self.cols.is_empty() || self.rows.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GreedyColumnError {
    MissingColumnCost { col: usize },
    MissingRowCost { row: usize },
    MissingMatrixValue { row: usize, col: usize },
}

impl fmt::Display for GreedyColumnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingColumnCost { col } => write!(f, "missing column cost for column {col}"),
            Self::MissingRowCost { row } => write!(f, "missing row cost for row {row}"),
            Self::MissingMatrixValue { row, col } => {
                write!(f, "missing matrix value for row {row}, column {col}")
            }
        }
    }
}

impl Error for GreedyColumnError {}

pub type GreedyColumnResult<T> = Result<T, GreedyColumnError>;

pub fn greedy_col(
    matrix: &GreedyColumnMatrix,
    col_cost: &[i32],
    row_cost: &[i32],
    seed_col: usize,
) -> GreedyColumnResult<GreedyRectangle> {
    let mut best_rect = GreedyRectangle::default();
    if matrix.is_empty() {
        return Ok(best_rect);
    }

    let Some(seed) = matrix.col(seed_col) else {
        return Ok(best_rect);
    };

    let mut rect = GreedyRectangle::default();
    rect.rows.extend(seed.keys().copied());
    rect.cols.insert(seed_col);

    let mut row_values = BTreeMap::new();
    for (&row, &value) in seed {
        row_values.insert(row, value - fetch_row_cost(row_cost, row)?);
    }

    let mut total_col_cost = fetch_col_cost(col_cost, seed_col)?;
    rect.value = -total_col_cost;
    for row in &rect.rows {
        rect.value += row_values.get(row).copied().unwrap_or_default();
    }

    let mut next_col = find_max_col_intersection(matrix, col_cost, &rect, &row_values)?;
    while let Some(col) = next_col {
        if rect.rows.len() <= 1 {
            break;
        }

        rect.cols.insert(col);
        total_col_cost += fetch_col_cost(col_cost, col)?;

        let selected_col_rows = matrix
            .col(col)
            .expect("selected column must exist in the matrix");
        rect.rows.retain(|row| selected_col_rows.contains_key(row));

        for (&row, &value) in selected_col_rows {
            *row_values.entry(row).or_default() += value;
        }

        rect.value = -total_col_cost;
        for row in &rect.rows {
            rect.value += row_values.get(row).copied().unwrap_or_default();
        }

        if rect.value > best_rect.value {
            best_rect = rect.clone();
        }

        debug_assert_eq!(
            check_rect_value(matrix, col_cost, row_cost, &rect)?,
            rect.value
        );
        next_col = find_max_col_intersection(matrix, col_cost, &rect, &row_values)?;
    }

    Ok(best_rect)
}

pub fn check_rect_value(
    matrix: &GreedyColumnMatrix,
    col_cost: &[i32],
    row_cost: &[i32],
    rect: &GreedyRectangle,
) -> GreedyColumnResult<i32> {
    let mut value = 0;
    for &col in &rect.cols {
        for &row in &rect.rows {
            value += matrix
                .value(row, col)
                .ok_or(GreedyColumnError::MissingMatrixValue { row, col })?;
        }
    }

    for &col in &rect.cols {
        value -= fetch_col_cost(col_cost, col)?;
    }

    for &row in &rect.rows {
        value -= fetch_row_cost(row_cost, row)?;
    }

    Ok(value)
}

fn find_max_col_intersection(
    matrix: &GreedyColumnMatrix,
    col_cost: &[i32],
    rect: &GreedyRectangle,
    row_values: &BTreeMap<usize, i32>,
) -> GreedyColumnResult<Option<usize>> {
    if matrix.is_empty() {
        return Ok(None);
    }

    let mut col_values = BTreeMap::new();
    for row in &rect.rows {
        if let Some(row_cols) = matrix.row(*row) {
            for col in row_cols.keys() {
                col_values.insert(*col, -fetch_col_cost(col_cost, *col)?);
            }
        }
    }

    for row in &rect.rows {
        if let Some(row_cols) = matrix.row(*row) {
            let row_value = row_values.get(row).copied().unwrap_or_default();
            for (&col, &value) in row_cols {
                *col_values.entry(col).or_default() += value + row_value;
            }
        }
    }

    let mut max_value = 0;
    let mut best_col = None;
    for row in &rect.rows {
        if let Some(row_cols) = matrix.row(*row) {
            for col in row_cols.keys() {
                let value = col_values.get(col).copied().unwrap_or_default();
                if value > max_value && !rect.cols.contains(col) {
                    max_value = value;
                    best_col = Some(*col);
                }
            }
        }
    }

    Ok(best_col)
}

fn fetch_col_cost(col_cost: &[i32], col: usize) -> GreedyColumnResult<i32> {
    col_cost
        .get(col)
        .copied()
        .ok_or(GreedyColumnError::MissingColumnCost { col })
}

fn fetch_row_cost(row_cost: &[i32], row: usize) -> GreedyColumnResult<i32> {
    row_cost
        .get(row)
        .copied()
        .ok_or(GreedyColumnError::MissingRowCost { row })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greedy_col_returns_empty_rectangle_for_empty_matrix() {
        let matrix = GreedyColumnMatrix::new();

        let rect = greedy_col(&matrix, &[], &[], 0).unwrap();

        assert_eq!(rect, GreedyRectangle::default());
    }

    #[test]
    fn greedy_col_returns_empty_rectangle_for_missing_seed_column() {
        let mut matrix = GreedyColumnMatrix::new();
        matrix.insert(0, 1, 3);

        let rect = greedy_col(&matrix, &[0, 1], &[1], 0).unwrap();

        assert_eq!(rect, GreedyRectangle::default());
    }

    #[test]
    fn greedy_col_selects_best_positive_intersecting_column() {
        let mut matrix = GreedyColumnMatrix::new();
        matrix.insert(0, 0, 5);
        matrix.insert(1, 0, 5);
        matrix.insert(2, 0, 1);
        matrix.insert(0, 1, 6);
        matrix.insert(1, 1, 6);
        matrix.insert(0, 2, 3);
        matrix.insert(2, 2, 10);

        let rect = greedy_col(&matrix, &[2, 1, 1], &[1, 1, 1], 0).unwrap();

        assert_eq!(rect.cols, BTreeSet::from([0, 1]));
        assert_eq!(rect.rows, BTreeSet::from([0, 1]));
        assert_eq!(rect.value, 17);
        assert_eq!(
            check_rect_value(&matrix, &[2, 1, 1], &[1, 1, 1], &rect).unwrap(),
            17
        );
    }

    #[test]
    fn greedy_col_can_keep_column_that_reduces_rectangle_to_one_row() {
        let mut matrix = GreedyColumnMatrix::new();
        matrix.insert(0, 0, 4);
        matrix.insert(1, 0, 4);
        matrix.insert(0, 1, 10);

        let rect = greedy_col(&matrix, &[1, 1], &[1, 1], 0).unwrap();

        assert_eq!(rect.cols, BTreeSet::from([0, 1]));
        assert_eq!(rect.rows, BTreeSet::from([0]));
        assert_eq!(rect.value, 11);
    }

    #[test]
    fn greedy_col_reports_missing_costs() {
        let mut matrix = GreedyColumnMatrix::new();
        matrix.insert(0, 0, 4);

        assert_eq!(
            greedy_col(&matrix, &[], &[1], 0),
            Err(GreedyColumnError::MissingColumnCost { col: 0 })
        );
        assert_eq!(
            greedy_col(&matrix, &[1], &[], 0),
            Err(GreedyColumnError::MissingRowCost { row: 0 })
        );
    }

    #[test]
    fn check_rect_value_reports_missing_matrix_entries() {
        let mut rect = GreedyRectangle::default();
        rect.rows.extend([0, 1]);
        rect.cols.insert(0);

        let mut matrix = GreedyColumnMatrix::new();
        matrix.insert(0, 0, 4);

        assert_eq!(
            check_rect_value(&matrix, &[1], &[1, 1], &rect),
            Err(GreedyColumnError::MissingMatrixValue { row: 1, col: 0 })
        );
    }
}
