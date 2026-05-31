//! Source-aligned replacement for Espresso's `matrix.c`.
//!
//! The C matrix owns row and column header arrays plus intrusive elements linked
//! in both directions.  `SparseMatrix` in `sparse.rs` replaces those structures
//! with `BTreeMap<usize, SparseRow>` and `BTreeMap<usize, SparseCol>`.  That
//! keeps the observable sparse matrix contract: unique `(row, col)` elements,
//! sorted row/column traversal, deletion of empty headers, duplication, counts,
//! longest row/column queries, and print/write formatting.

pub use crate::sparse::SparseMatrix;

use crate::sparse::{SparseCol, SparseRow};

/// Rust replacement for `sm_alloc`.
pub fn sm_alloc() -> SparseMatrix {
    SparseMatrix::new()
}

/// Rust replacement for `sm_alloc_size`.
pub fn sm_alloc_size(row: usize, col: usize) -> SparseMatrix {
    SparseMatrix::with_size(row, col)
}

/// Rust replacement for `sm_dup`.
pub fn sm_dup(matrix: &SparseMatrix) -> SparseMatrix {
    matrix.clone()
}

/// Rust replacement for `sm_insert`.
pub fn sm_insert(matrix: &mut SparseMatrix, row: usize, col: usize) -> bool {
    matrix.insert(row, col)
}

/// Rust replacement for `sm_find`.
pub fn sm_find(matrix: &SparseMatrix, row: usize, col: usize) -> bool {
    matrix.contains(row, col)
}

/// Rust replacement for `sm_remove`.
pub fn sm_remove(matrix: &mut SparseMatrix, row: usize, col: usize) -> bool {
    matrix.remove(row, col)
}

/// Rust replacement for `sm_delrow`.
pub fn sm_delrow(matrix: &mut SparseMatrix, row: usize) -> bool {
    matrix.delete_row(row)
}

/// Rust replacement for `sm_delcol`.
pub fn sm_delcol(matrix: &mut SparseMatrix, col: usize) -> bool {
    matrix.delete_col(col)
}

/// Rust replacement for `sm_copy_row`.
pub fn sm_copy_row(dest: &mut SparseMatrix, dest_row: usize, row: &SparseRow) {
    dest.copy_row_from(dest_row, row);
}

/// Rust replacement for `sm_copy_col`.
pub fn sm_copy_col(dest: &mut SparseMatrix, dest_col: usize, col: &SparseCol) {
    dest.copy_col_from(dest_col, col);
}

/// Rust replacement for `sm_longest_row`.
pub fn sm_longest_row(matrix: &SparseMatrix) -> Option<&SparseRow> {
    matrix.longest_row()
}

/// Rust replacement for `sm_longest_col`.
pub fn sm_longest_col(matrix: &SparseMatrix) -> Option<&SparseCol> {
    matrix.longest_col()
}

/// Rust replacement for `sm_num_elements`.
pub fn sm_num_elements(matrix: &SparseMatrix) -> usize {
    matrix.num_elements()
}

/// Rust replacement for `sm_write`.
pub fn sm_write(matrix: &SparseMatrix) -> String {
    matrix
        .to_pairs()
        .into_iter()
        .map(|(row, col)| format!("{row} {col}\n"))
        .collect()
}

/// Rust replacement for `sm_print`.
pub fn sm_print(matrix: &SparseMatrix) -> String {
    matrix.print_matrix()
}

/// Rust replacement for `sm_read` over already-tokenized pairs.
pub fn sm_read_pairs(pairs: impl IntoIterator<Item = (usize, usize)>) -> SparseMatrix {
    let mut matrix = SparseMatrix::new();
    for (row, col) in pairs {
        matrix.insert(row, col);
    }
    matrix
}

/// Rust replacement for `sm_read_compressed` over parsed row labels and words.
///
/// Espresso's C reader consumes one row label word and then enough 32-bit words
/// to cover the declared column count.  Set bits insert `(row_index, col)`.
pub fn sm_read_compressed_words(
    nrows: usize,
    ncols: usize,
    rows: impl IntoIterator<Item = (usize, Vec<u64>)>,
) -> SparseMatrix {
    let mut matrix = SparseMatrix::with_size(nrows, ncols);
    let words_per_row = ncols.div_ceil(32);

    for (row_index, (_row_label, words)) in rows.into_iter().take(nrows).enumerate() {
        for (word_index, mut word) in words.into_iter().take(words_per_row).enumerate() {
            let base_col = word_index * 32;
            while word != 0 {
                let bit = word.trailing_zeros() as usize;
                let col = base_col + bit;
                if col < ncols {
                    matrix.insert(row_index, col);
                }
                word &= word - 1;
            }
        }
    }

    matrix
}

/// Rust replacement for `sm_dump`.
pub fn sm_dump(matrix: &SparseMatrix, label: &str, max: usize) -> String {
    let mut out = format!(
        "{label} {} rows by {} cols\n",
        matrix.nrows(),
        matrix.ncols()
    );
    if matrix.nrows() < max {
        out.push_str(&matrix.print_matrix());
    }
    out
}

/// Rust replacement for `sm_cleanup`.
///
/// The C implementation only releases optional freelists.  Rust has no sparse
/// matrix freelists, so cleanup is intentionally a no-op.
pub fn sm_cleanup() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_insert_find_remove_and_delete_match_matrix_c() {
        let mut matrix = sm_alloc();

        assert!(sm_insert(&mut matrix, 2, 4));
        assert!(sm_insert(&mut matrix, 0, 3));
        assert!(sm_insert(&mut matrix, 2, 1));
        assert!(!sm_insert(&mut matrix, 0, 3));
        assert_eq!(matrix.to_pairs(), vec![(0, 3), (2, 1), (2, 4)]);
        assert_eq!(sm_num_elements(&matrix), 3);
        assert!(sm_find(&matrix, 2, 1));

        assert!(sm_remove(&mut matrix, 2, 1));
        assert!(!sm_find(&matrix, 2, 1));
        assert!(matrix.col(1).is_none());

        assert!(sm_delcol(&mut matrix, 4));
        assert!(matrix.row(2).is_none());
        assert!(sm_delrow(&mut matrix, 0));
        assert!(matrix.is_empty());
    }

    #[test]
    fn matrix_copy_longest_write_and_print_match_matrix_c_contract() {
        let source = sm_read_pairs([(0, 1), (0, 3), (2, 3), (2, 4), (2, 9)]);
        let mut dest = sm_alloc_size(10, 10);

        sm_copy_row(&mut dest, 7, source.row(2).unwrap());
        sm_copy_col(&mut dest, 8, source.col(3).unwrap());

        assert_eq!(
            dest.to_pairs(),
            vec![(0, 8), (2, 8), (7, 3), (7, 4), (7, 9)]
        );
        assert_eq!(sm_longest_row(&dest).unwrap().row_num, 7);
        assert_eq!(sm_longest_col(&dest).unwrap().col_num, 8);
        assert_eq!(sm_write(&dest), "0 8\n2 8\n7 3\n7 4\n7 9\n");
        assert_eq!(
            sm_print(&dest),
            "    3489\n    ----\n  0:..1.\n  2:..1.\n  7:11.1\n"
        );
    }

    #[test]
    fn matrix_dup_is_independent() {
        let mut original = sm_read_pairs([(0, 0), (1, 1)]);
        let duplicate = sm_dup(&original);

        sm_remove(&mut original, 0, 0);

        assert_eq!(original.to_pairs(), vec![(1, 1)]);
        assert_eq!(duplicate.to_pairs(), vec![(0, 0), (1, 1)]);
    }

    #[test]
    fn matrix_compressed_read_dump_and_cleanup_cover_c_helpers() {
        let matrix = sm_read_compressed_words(2, 35, [(0, vec![0b101, 0b10]), (1, vec![0, 0b1])]);

        assert_eq!(matrix.to_pairs(), vec![(0, 0), (0, 2), (0, 33), (1, 32)]);
        assert_eq!(
            sm_dump(&matrix, "compressed", 10),
            "compressed 2 rows by 4 cols\n    0033\n    0223\n    ----\n  0:11.1\n  1:..1.\n"
        );
        sm_cleanup();
    }
}
