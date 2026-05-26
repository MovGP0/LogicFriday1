//! Native Rust port of `sis/extract/rect.c`.
//!
//! The legacy implementation recursively enumerates canonical rectangles in a
//! sparse matrix and lets a caller-provided callback decide whether each
//! recorded rectangle should be expanded further. This port keeps that behavior
//! over owned Rust sparse matrices and rectangle records.

use crate::ports::sparse::matrix::SparseMatrix;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Rectangle {
    rows: BTreeSet<usize>,
    cols: BTreeSet<usize>,
    value: i32,
}

impl Rectangle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parts(
        rows: impl IntoIterator<Item = usize>,
        cols: impl IntoIterator<Item = usize>,
    ) -> Self {
        Self {
            rows: rows.into_iter().collect(),
            cols: cols.into_iter().collect(),
            value: 0,
        }
    }

    pub fn rows(&self) -> &BTreeSet<usize> {
        &self.rows
    }

    pub fn cols(&self) -> &BTreeSet<usize> {
        &self.cols
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn set_value(&mut self, value: i32) {
        self.value = value;
    }
}

pub fn generate_all_rectangles(matrix: &SparseMatrix) -> Vec<Rectangle> {
    let mut rectangles = Vec::new();
    generate_all_rectangles_with(matrix, |_, rectangle| {
        rectangles.push(rectangle.clone());
        true
    });
    rectangles
}

pub fn generate_all_rectangles_with(
    matrix: &SparseMatrix,
    mut record: impl FnMut(&SparseMatrix, &Rectangle) -> bool,
) {
    let rectangle = Rectangle::new();
    generate_all_rectangles_from(matrix, &rectangle, 0, &mut record);

    if !has_full_column(matrix) {
        record(matrix, &rectangle);
    }
}

fn generate_all_rectangles_from(
    matrix: &SparseMatrix,
    rectangle: &Rectangle,
    min_col: usize,
    record: &mut impl FnMut(&SparseMatrix, &Rectangle) -> bool,
) {
    let columns = matrix.cols().collect::<Vec<_>>();

    for column in columns {
        if column.len() < 2 || column.index() < min_col {
            continue;
        }

        let mut submatrix = rows_induced_by_column(matrix, column.elements());
        let mut subrectangle = Rectangle {
            rows: column.elements().iter().copied().collect(),
            cols: rectangle.cols.clone(),
            value: 0,
        };

        let mut already_generated = false;
        let full_columns = submatrix.cols().collect::<Vec<_>>();
        for full_column in full_columns {
            if full_column.len() == column.len() {
                if full_column.index() < column.index() {
                    already_generated = true;
                    break;
                }

                subrectangle.cols.insert(full_column.index());
                submatrix.delete_col(full_column.index());
            }
        }

        if !already_generated && record(&submatrix, &subrectangle) {
            generate_all_rectangles_from(&submatrix, &subrectangle, column.index(), record);
        }
    }
}

fn rows_induced_by_column(matrix: &SparseMatrix, rows: &[usize]) -> SparseMatrix {
    let mut submatrix = SparseMatrix::with_size(matrix.rows_size(), matrix.cols_size());
    for row in rows {
        if let Some(source_row) = matrix.row(*row) {
            submatrix.copy_row_from(*row, &source_row);
        }
    }

    submatrix
}

fn has_full_column(matrix: &SparseMatrix) -> bool {
    let row_count = matrix.row_count();
    matrix.cols().any(|column| column.len() == row_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_empty_rectangle_when_matrix_has_no_full_column() {
        let matrix = matrix_from_pairs(&[(0, 0), (1, 1)]);

        let rectangles = generate_all_rectangles(&matrix);

        assert_eq!(rectangles, vec![Rectangle::new()]);
    }

    #[test]
    fn skips_empty_rectangle_when_original_matrix_has_full_column() {
        let matrix = matrix_from_pairs(&[(0, 0), (1, 0), (1, 1)]);

        let rectangles = generate_all_rectangles(&matrix);

        assert_eq!(rectangles, vec![Rectangle::with_parts([0, 1], [0])]);
    }

    #[test]
    fn factors_all_columns_full_over_selected_rows() {
        let matrix = matrix_from_pairs(&[(0, 1), (0, 2), (0, 4), (1, 1), (1, 2), (2, 2), (2, 3)]);

        let mut snapshots = Vec::new();
        generate_all_rectangles_with(&matrix, |submatrix, rectangle| {
            snapshots.push((
                rectangle.clone(),
                submatrix
                    .elements()
                    .map(|element| (element.row, element.col))
                    .collect::<Vec<_>>(),
            ));
            true
        });

        assert_eq!(
            snapshots[0],
            (Rectangle::with_parts([0, 1], [1, 2]), vec![(0, 4)])
        );
    }

    #[test]
    fn suppresses_rectangles_already_generated_from_earlier_columns() {
        let matrix = matrix_from_pairs(&[(0, 1), (0, 2), (1, 1), (1, 2)]);

        let rectangles = generate_all_rectangles(&matrix);

        assert_eq!(rectangles, vec![Rectangle::with_parts([0, 1], [1, 2])]);
    }

    #[test]
    fn record_return_value_controls_recursive_expansion() {
        let matrix = matrix_from_pairs(&[(0, 1), (0, 2), (0, 3), (1, 1), (1, 2), (2, 1), (2, 3)]);
        let mut rectangles = Vec::new();

        generate_all_rectangles_with(&matrix, |_, rectangle| {
            rectangles.push(rectangle.clone());
            false
        });

        assert_eq!(rectangles, vec![Rectangle::with_parts([0, 1, 2], [1])]);
    }

    #[test]
    fn recurses_from_recorded_rectangles_when_callback_allows_it() {
        let matrix = matrix_from_pairs(&[(0, 1), (0, 2), (0, 3), (1, 1), (1, 2), (2, 1), (2, 3)]);

        let rectangles = generate_all_rectangles(&matrix);

        assert_eq!(
            rectangles,
            vec![
                Rectangle::with_parts([0, 1, 2], [1]),
                Rectangle::with_parts([0, 1], [1, 2]),
                Rectangle::with_parts([0, 2], [1, 3])
            ]
        );
    }

    #[test]
    fn clone_preserves_rectangle_contents_like_rect_dup() {
        let mut rectangle = Rectangle::with_parts([2, 0], [5, 1]);
        rectangle.set_value(17);

        let clone = rectangle.clone();
        rectangle.set_value(3);

        assert_eq!(clone.rows(), &BTreeSet::from([0, 2]));
        assert_eq!(clone.cols(), &BTreeSet::from([1, 5]));
        assert_eq!(clone.value(), 17);
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_metadata_tokens_are_present_in_this_port() {
        let source = include_str!("rect.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("b", "ead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday", "1-", "8j8")));
    }

    fn matrix_from_pairs(pairs: &[(usize, usize)]) -> SparseMatrix {
        let mut matrix = SparseMatrix::new();
        for (row, col) in pairs {
            matrix.insert(*row, *col);
        }

        matrix
    }
}
