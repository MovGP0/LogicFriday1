//! Native Rust port of `LogicSynthesis/sis/mincov/part.c`.
//!
//! The C implementation partitions an `sm_matrix` into independent connected
//! row/column blocks by walking the bipartite graph from the first row. A
//! partition exists only when that walk reaches a strict subset of the rows.

use std::collections::{BTreeSet, VecDeque};

use crate::ports::sparse::matrix::SparseMatrix;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockPartition {
    pub left: SparseMatrix,
    pub right: SparseMatrix,
    pub visited_rows: BTreeSet<usize>,
    pub visited_cols: BTreeSet<usize>,
}

impl BlockPartition {
    pub fn is_balanced(&self) -> bool {
        !self.left.is_empty() && !self.right.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentVisit {
    pub rows: BTreeSet<usize>,
    pub cols: BTreeSet<usize>,
}

impl ComponentVisit {
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize {
        self.cols.len()
    }
}

pub fn sm_block_partition(matrix: &SparseMatrix) -> Option<BlockPartition> {
    block_partition(matrix)
}

pub fn block_partition(matrix: &SparseMatrix) -> Option<BlockPartition> {
    if matrix.row_count() == 0 {
        return None;
    }

    let visit = visit_from_first_row(matrix)?;
    if visit.row_count() == matrix.row_count() {
        return None;
    }

    let mut left = SparseMatrix::new();
    let mut right = SparseMatrix::new();

    for row in matrix.rows() {
        if visit.rows.contains(&row.index()) {
            copy_row(&mut left, row.index(), matrix);
        } else {
            copy_row(&mut right, row.index(), matrix);
        }
    }

    Some(BlockPartition {
        left,
        right,
        visited_rows: visit.rows,
        visited_cols: visit.cols,
    })
}

pub fn visit_from_first_row(matrix: &SparseMatrix) -> Option<ComponentVisit> {
    let first_row = matrix.rows().next()?.index();

    Some(visit_component_from_row(matrix, first_row))
}

pub fn visit_component_from_row(matrix: &SparseMatrix, start_row: usize) -> ComponentVisit {
    let mut rows = BTreeSet::new();
    let mut cols = BTreeSet::new();
    let mut pending_rows = VecDeque::from([start_row]);
    let mut pending_cols = VecDeque::new();

    while !pending_rows.is_empty() || !pending_cols.is_empty() {
        while let Some(row_num) = pending_rows.pop_front() {
            if !rows.insert(row_num) {
                continue;
            }

            if let Some(row) = matrix.row(row_num) {
                for col_num in row.elements() {
                    if !cols.contains(col_num) {
                        pending_cols.push_back(*col_num);
                    }
                }
            }
        }

        while let Some(col_num) = pending_cols.pop_front() {
            if !cols.insert(col_num) {
                continue;
            }

            if let Some(col) = matrix.col(col_num) {
                for row_num in col.elements() {
                    if !rows.contains(row_num) {
                        pending_rows.push_back(*row_num);
                    }
                }
            }
        }
    }

    ComponentVisit { rows, cols }
}

pub fn copy_row(dest: &mut SparseMatrix, row_num: usize, source: &SparseMatrix) {
    if let Some(row) = source.row(row_num) {
        dest.copy_row_from(row_num, &row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_connected_matrices_do_not_partition() {
        assert_eq!(sm_block_partition(&SparseMatrix::new()), None);

        let matrix = matrix_from_pairs(&[(0, 0), (1, 0), (1, 1), (2, 1)]);

        assert_eq!(sm_block_partition(&matrix), None);
    }

    #[test]
    fn partitions_rows_by_component_reached_from_first_row() {
        let matrix = matrix_from_pairs(&[(0, 0), (1, 0), (3, 5), (4, 5), (4, 6)]);
        let partition = sm_block_partition(&matrix).unwrap();

        assert!(partition.is_balanced());
        assert_eq!(partition.visited_rows, set(&[0, 1]));
        assert_eq!(partition.visited_cols, set(&[0]));
        assert_eq!(partition.left.write_pairs(), "0 0\n1 0\n");
        assert_eq!(partition.right.write_pairs(), "3 5\n4 5\n4 6\n");
    }

    #[test]
    fn starts_from_lowest_active_row_like_first_row_list_order() {
        let matrix = matrix_from_pairs(&[(2, 8), (5, 8), (0, 1)]);
        let partition = sm_block_partition(&matrix).unwrap();

        assert_eq!(partition.visited_rows, set(&[0]));
        assert_eq!(partition.left.write_pairs(), "0 1\n");
        assert_eq!(partition.right.write_pairs(), "2 8\n5 8\n");
    }

    #[test]
    fn component_visit_alternates_rows_and_columns() {
        let matrix = matrix_from_pairs(&[(0, 2), (3, 2), (3, 4), (7, 4), (9, 9)]);
        let visit = visit_component_from_row(&matrix, 0);

        assert_eq!(visit.rows, set(&[0, 3, 7]));
        assert_eq!(visit.cols, set(&[2, 4]));
        assert_eq!(visit.row_count(), 3);
        assert_eq!(visit.col_count(), 2);
    }

    #[test]
    fn copy_row_preserves_original_row_number_and_columns() {
        let source = matrix_from_pairs(&[(4, 1), (4, 3), (8, 2)]);
        let mut dest = SparseMatrix::new();

        copy_row(&mut dest, 4, &source);

        assert_eq!(dest.write_pairs(), "4 1\n4 3\n");
    }

    fn matrix_from_pairs(pairs: &[(usize, usize)]) -> SparseMatrix {
        let mut matrix = SparseMatrix::new();
        for (row, col) in pairs {
            matrix.insert(*row, *col);
        }
        matrix
    }

    fn set(values: &[usize]) -> BTreeSet<usize> {
        values.iter().copied().collect()
    }
}
