//! Source-aligned replacement for Espresso's `rows.c`.
//!
//! The C file implements an intrusive sorted linked-list row vector.  In the
//! Rust port, `SparseRow` stores column numbers in a `BTreeSet` inside
//! `sparse.rs`.  This preserves sorted unique elements and the row-vector
//! operations used by Espresso while relying on Rust ownership instead of
//! freelists and raw linked nodes.

pub use crate::sparse::SparseRow;

/// Rust replacement for `sm_row_alloc`.
pub fn sm_row_alloc(row_num: usize) -> SparseRow {
    SparseRow::new(row_num)
}

/// Rust replacement for `sm_row_dup`.
pub fn sm_row_dup(row: &SparseRow) -> SparseRow {
    row.clone()
}

/// Rust replacement for `sm_row_insert`.
pub fn sm_row_insert(row: &mut SparseRow, col: usize) -> bool {
    row.insert(col)
}

/// Rust replacement for `sm_row_remove`.
pub fn sm_row_remove(row: &mut SparseRow, col: usize) -> bool {
    row.remove(col)
}

/// Rust replacement for `sm_row_find`.
pub fn sm_row_find(row: &SparseRow, col: usize) -> bool {
    row.contains_col(col)
}

/// Rust replacement for `sm_row_contains`: returns true when `p2` contains `p1`.
pub fn sm_row_contains(p1: &SparseRow, p2: &SparseRow) -> bool {
    p1.is_contained_by(p2)
}

/// Rust replacement for `sm_row_intersects`.
pub fn sm_row_intersects(p1: &SparseRow, p2: &SparseRow) -> bool {
    p1.intersects(p2)
}

/// Rust replacement for `sm_row_compare`.
pub fn sm_row_compare(p1: &SparseRow, p2: &SparseRow) -> i32 {
    p1.compare_lexicographic(p2)
}

/// Rust replacement for `sm_row_and`.
pub fn sm_row_and(p1: &SparseRow, p2: &SparseRow) -> SparseRow {
    p1.intersection(p2)
}

/// Rust replacement for `sm_row_hash`.
pub fn sm_row_hash(row: &SparseRow, modulus: usize) -> usize {
    row.hash_mod(modulus)
}

/// Rust replacement for `sm_row_print`.
pub fn sm_row_print(row: &SparseRow) -> String {
    row.cols().map(|col| format!(" {col}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_from_cols(row_num: usize, cols: impl IntoIterator<Item = usize>) -> SparseRow {
        let mut row = sm_row_alloc(row_num);
        for col in cols {
            sm_row_insert(&mut row, col);
        }
        row
    }

    #[test]
    fn row_vector_matches_rows_c_insert_find_remove_behavior() {
        let mut row = row_from_cols(2, [5, 1, 5, 3]);
        let dup = sm_row_dup(&row);

        assert_eq!(dup, row);
        assert_eq!(row.cols().collect::<Vec<_>>(), vec![1, 3, 5]);
        assert!(sm_row_find(&row, 3));
        assert!(!sm_row_find(&row, 4));
        assert!(!sm_row_insert(&mut row, 3));
        assert!(sm_row_remove(&mut row, 3));
        assert!(!sm_row_remove(&mut row, 3));
        assert_eq!(sm_row_print(&row), " 1 5");
    }

    #[test]
    fn row_contains_intersects_compare_and_hash_match_rows_c() {
        let subset = row_from_cols(0, [1, 7]);
        let superset = row_from_cols(0, [1, 3, 7]);
        let disjoint = row_from_cols(0, [2, 4]);

        assert!(sm_row_contains(&subset, &superset));
        assert!(!sm_row_contains(&superset, &subset));
        assert!(sm_row_intersects(&subset, &superset));
        assert!(!sm_row_intersects(&subset, &disjoint));
        assert!(sm_row_compare(&subset, &superset) > 0);
        assert_eq!(
            sm_row_and(&subset, &superset).cols().collect::<Vec<_>>(),
            vec![1, 7]
        );
        assert_eq!(sm_row_hash(&superset, 97), (((1 * 17 + 3) * 17 + 7) % 97));
    }
}
