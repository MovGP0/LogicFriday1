//! Source-aligned replacement for Espresso's `cols.c`.
//!
//! The C file implements an intrusive sorted linked-list column vector.  In the
//! Rust port, `SparseCol` stores row numbers in a `BTreeSet` inside
//! `sparse.rs`.  That gives the same uniqueness, deterministic sorted
//! traversal, subset, intersection, lexical compare, and hash behavior without
//! exposing pointer ownership.  Standalone column mutation is intentionally not
//! public: columns that belong to a matrix must be updated through
//! `SparseMatrix` so row and column indexes remain consistent.

pub use crate::sparse::SparseCol;

use crate::sparse::SparseMatrix;

/// Rust replacement for `sm_col_alloc`.
pub fn sm_col_alloc(col_num: usize) -> SparseCol {
    SparseCol::new(col_num)
}

/// Rust replacement for `sm_col_dup`.
pub fn sm_col_dup(col: &SparseCol) -> SparseCol {
    col.clone()
}

/// Builds a column vector by inserting rows through `SparseMatrix`.
///
/// This replaces standalone `sm_col_insert` for source-aligned tests and
/// callers that need an owned column value.  Matrix-owned columns should still
/// be changed with `SparseMatrix::insert`/`SparseMatrix::remove`.
pub fn sm_col_from_rows(col_num: usize, rows: impl IntoIterator<Item = usize>) -> SparseCol {
    let mut matrix = SparseMatrix::new();
    for row in rows {
        matrix.insert(row, col_num);
    }
    matrix
        .col(col_num)
        .cloned()
        .unwrap_or_else(|| SparseCol::new(col_num))
}

/// Rust replacement for `sm_col_find`.
pub fn sm_col_find(col: &SparseCol, row: usize) -> bool {
    col.contains_row(row)
}

/// Rust replacement for `sm_col_contains`: returns true when `p2` contains `p1`.
pub fn sm_col_contains(p1: &SparseCol, p2: &SparseCol) -> bool {
    p1.is_contained_by(p2)
}

/// Rust replacement for `sm_col_intersects`.
pub fn sm_col_intersects(p1: &SparseCol, p2: &SparseCol) -> bool {
    p1.intersects(p2)
}

/// Rust replacement for `sm_col_compare`.
pub fn sm_col_compare(p1: &SparseCol, p2: &SparseCol) -> i32 {
    p1.compare_lexicographic(p2)
}

/// Rust replacement for `sm_col_and`.
pub fn sm_col_and(p1: &SparseCol, p2: &SparseCol) -> SparseCol {
    p1.intersection(p2)
}

/// Rust replacement for `sm_col_hash`.
pub fn sm_col_hash(col: &SparseCol, modulus: usize) -> usize {
    col.hash_mod(modulus)
}

/// Rust replacement for `sm_col_print`.
pub fn sm_col_print(col: &SparseCol) -> String {
    col.rows().map(|row| format!(" {row}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn col_vector_matches_cols_c_sorted_set_behavior() {
        let col = sm_col_from_rows(4, [3, 1, 3, 7]);
        let empty = sm_col_alloc(9);
        let dup = sm_col_dup(&col);

        assert_eq!(empty.col_num, 9);
        assert!(empty.is_empty());
        assert_eq!(dup, col);
        assert_eq!(col.col_num, 4);
        assert_eq!(col.rows().collect::<Vec<_>>(), vec![1, 3, 7]);
        assert!(sm_col_find(&col, 3));
        assert!(!sm_col_find(&col, 2));
        assert_eq!(sm_col_print(&col), " 1 3 7");
    }

    #[test]
    fn col_contains_intersects_compare_and_hash_match_cols_c() {
        let subset = sm_col_from_rows(0, [1, 7]);
        let superset = sm_col_from_rows(0, [1, 3, 7]);
        let disjoint = sm_col_from_rows(0, [2, 4]);

        assert!(sm_col_contains(&subset, &superset));
        assert!(!sm_col_contains(&superset, &subset));
        assert!(sm_col_intersects(&subset, &superset));
        assert!(!sm_col_intersects(&subset, &disjoint));
        assert!(sm_col_compare(&subset, &superset) > 0);
        assert_eq!(
            sm_col_and(&subset, &superset).rows().collect::<Vec<_>>(),
            vec![1, 7]
        );
        assert_eq!(sm_col_hash(&superset, 97), (((1 * 17 + 3) * 17 + 7) % 97));
    }
}
