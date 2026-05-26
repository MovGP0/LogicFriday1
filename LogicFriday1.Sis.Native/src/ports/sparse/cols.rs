//! Native Rust port of `LogicSynthesis/sis/sparse/cols.c`.
//!
//! The C module stores each sparse-matrix column as a sorted doubly linked list
//! of `sm_element` entries keyed by `row_num`. This Rust port keeps the same
//! observable column-vector behavior with an owned, sorted, duplicate-free
//! vector of row numbers. C allocation/free-list routines are intentionally
//! retired in favor of Rust ownership.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

pub const REQUIRED_INTEGRATION_PORT_BEADS: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.457",
        source_file: "LogicSynthesis/sis/sparse/matrix.c",
        note: "matrix-owned insertion/removal must keep row and column views synchronized",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.458",
        source_file: "LogicSynthesis/sis/sparse/rows.c",
        note: "row-vector counterpart is needed for full sparse-matrix integration",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub note: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SparseColumn {
    col_num: i32,
    flag: i32,
    user_word: Option<String>,
    rows: Vec<i32>,
}

impl SparseColumn {
    pub fn new() -> Self {
        Self {
            col_num: 0,
            flag: 0,
            user_word: None,
            rows: Vec::new(),
        }
    }

    pub fn with_col_num(col_num: i32) -> Self {
        Self {
            col_num,
            ..Self::new()
        }
    }

    pub fn col_num(&self) -> i32 {
        self.col_num
    }

    pub fn set_col_num(&mut self, col_num: i32) {
        self.col_num = col_num;
    }

    pub fn flag(&self) -> i32 {
        self.flag
    }

    pub fn set_flag(&mut self, flag: i32) {
        self.flag = flag;
    }

    pub fn user_word(&self) -> Option<&str> {
        self.user_word.as_deref()
    }

    pub fn set_user_word(&mut self, user_word: Option<String>) {
        self.user_word = user_word;
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn rows(&self) -> &[i32] {
        &self.rows
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = i32> + '_ {
        self.rows.iter().copied()
    }

    pub fn insert(&mut self, row: i32) -> ColumnElement {
        match self.rows.binary_search(&row) {
            Ok(index) => ColumnElement {
                row_num: self.rows[index],
                inserted: false,
            },
            Err(index) => {
                self.rows.insert(index, row);
                ColumnElement {
                    row_num: row,
                    inserted: true,
                }
            }
        }
    }

    pub fn remove(&mut self, row: i32) -> bool {
        let Ok(index) = self.rows.binary_search(&row) else {
            return false;
        };

        self.rows.remove(index);
        true
    }

    pub fn remove_element(&mut self, element: ColumnElement) -> bool {
        self.remove(element.row_num)
    }

    pub fn find(&self, row: i32) -> Option<ColumnElement> {
        self.rows
            .binary_search(&row)
            .ok()
            .map(|index| ColumnElement {
                row_num: self.rows[index],
                inserted: false,
            })
    }

    pub fn contains_column(&self, subset: &Self) -> bool {
        let mut superset = self.rows.iter();
        let mut next_super = superset.next();

        for row in &subset.rows {
            loop {
                match next_super {
                    Some(candidate) if candidate < row => next_super = superset.next(),
                    Some(candidate) if candidate == row => {
                        next_super = superset.next();
                        break;
                    }
                    _ => return false,
                }
            }
        }

        true
    }

    pub fn intersects(&self, other: &Self) -> bool {
        let mut left = 0;
        let mut right = 0;

        while left < self.rows.len() && right < other.rows.len() {
            match self.rows[left].cmp(&other.rows[right]) {
                Ordering::Less => left += 1,
                Ordering::Greater => right += 1,
                Ordering::Equal => return true,
            }
        }

        false
    }

    pub fn sis_compare(&self, other: &Self) -> i32 {
        let mut left = self.rows.iter();
        let mut right = other.rows.iter();

        loop {
            match (left.next(), right.next()) {
                (Some(a), Some(b)) if a != b => return a - b,
                (Some(_), Some(_)) => {}
                (Some(_), None) => return 1,
                (None, Some(_)) => return -1,
                (None, None) => return 0,
            }
        }
    }

    pub fn intersection(&self, other: &Self) -> Self {
        let mut result = Self::new();
        let mut left = 0;
        let mut right = 0;

        while left < self.rows.len() && right < other.rows.len() {
            match self.rows[left].cmp(&other.rows[right]) {
                Ordering::Less => left += 1,
                Ordering::Greater => right += 1,
                Ordering::Equal => {
                    result.rows.push(self.rows[left]);
                    left += 1;
                    right += 1;
                }
            }
        }

        result
    }

    pub fn sis_hash(&self, modulus: i32) -> Result<i32, SparseColumnError> {
        if modulus <= 0 {
            return Err(SparseColumnError::InvalidHashModulus(modulus));
        }

        Ok(self
            .rows
            .iter()
            .fold(0, |sum, row| (sum * 17 + row) % modulus))
    }

    pub fn print_body(&self) -> String {
        self.rows
            .iter()
            .map(|row| format!(" {row}"))
            .collect::<String>()
    }

    pub fn full_sparse_matrix_integration() -> Result<(), SparseColumnError> {
        Err(SparseColumnError::MissingSparseMatrixPorts {
            dependencies: REQUIRED_INTEGRATION_PORT_BEADS,
        })
    }
}

impl Default for SparseColumn {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<i32> for SparseColumn {
    fn from_iter<T: IntoIterator<Item = i32>>(iter: T) -> Self {
        let mut column = Self::new();
        for row in iter {
            column.insert(row);
        }
        column
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ColumnElement {
    row_num: i32,
    inserted: bool,
}

impl ColumnElement {
    pub fn row_num(self) -> i32 {
        self.row_num
    }

    pub fn was_inserted(self) -> bool {
        self.inserted
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SparseColumnError {
    InvalidHashModulus(i32),
    MissingSparseMatrixPorts {
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for SparseColumnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHashModulus(modulus) => {
                write!(f, "invalid sparse-column hash modulus {modulus}")
            }
            Self::MissingSparseMatrixPorts { dependencies } => write!(
                f,
                "full sparse-column matrix integration is blocked by {} unported dependencies",
                dependencies.len()
            ),
        }
    }
}

impl Error for SparseColumnError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(rows: &[i32]) -> SparseColumn {
        rows.iter().copied().collect()
    }

    #[test]
    fn starts_empty_with_sis_defaults() {
        let column = SparseColumn::new();

        assert_eq!(column.col_num(), 0);
        assert_eq!(column.flag(), 0);
        assert_eq!(column.user_word(), None);
        assert!(column.is_empty());
        assert_eq!(column.print_body(), "");
    }

    #[test]
    fn insert_keeps_rows_sorted_and_unique() {
        let mut column = SparseColumn::new();

        assert!(column.insert(7).was_inserted());
        assert!(column.insert(2).was_inserted());
        assert!(column.insert(5).was_inserted());
        assert!(!column.insert(5).was_inserted());

        assert_eq!(column.rows(), &[2, 5, 7]);
        assert_eq!(column.len(), 3);
    }

    #[test]
    fn find_and_remove_match_c_column_semantics() {
        let mut column = column(&[1, 3, 9]);

        assert_eq!(column.find(3).map(ColumnElement::row_num), Some(3));
        assert_eq!(column.find(4), None);
        assert!(column.remove(3));
        assert!(!column.remove(3));
        assert_eq!(column.rows(), &[1, 9]);

        let element = column.find(9).expect("row 9 is present");
        assert!(column.remove_element(element));
        assert_eq!(column.rows(), &[1]);
    }

    #[test]
    fn contains_tests_subset_relationship() {
        let superset = column(&[1, 2, 4, 9]);
        let subset = column(&[2, 9]);
        let missing = column(&[2, 8]);

        assert!(superset.contains_column(&subset));
        assert!(superset.contains_column(&SparseColumn::new()));
        assert!(!subset.contains_column(&superset));
        assert!(!superset.contains_column(&missing));
    }

    #[test]
    fn intersects_uses_sorted_merge_walk() {
        assert!(column(&[1, 4, 8]).intersects(&column(&[2, 4, 9])));
        assert!(!column(&[1, 3]).intersects(&column(&[2, 4])));
        assert!(!SparseColumn::new().intersects(&column(&[1])));
    }

    #[test]
    fn sis_compare_matches_cols_c_return_shape() {
        assert_eq!(column(&[1, 5]).sis_compare(&column(&[1, 7])), -2);
        assert_eq!(column(&[1, 7]).sis_compare(&column(&[1, 5])), 2);
        assert_eq!(column(&[1, 5]).sis_compare(&column(&[1, 5])), 0);
        assert_eq!(column(&[1, 5, 9]).sis_compare(&column(&[1, 5])), 1);
        assert_eq!(column(&[1, 5]).sis_compare(&column(&[1, 5, 9])), -1);
    }

    #[test]
    fn intersection_returns_new_column() {
        let result = column(&[1, 3, 5, 7]).intersection(&column(&[2, 3, 7, 11]));

        assert_eq!(result.rows(), &[3, 7]);
    }

    #[test]
    fn hash_uses_sis_recurrence() {
        let column = column(&[2, 5, 11]);

        assert_eq!(column.sis_hash(97), Ok((((2 * 17) + 5) * 17 + 11) % 97));
        assert_eq!(
            column.sis_hash(0),
            Err(SparseColumnError::InvalidHashModulus(0))
        );
    }

    #[test]
    fn print_body_matches_c_spacing() {
        assert_eq!(column(&[2, 5, 11]).print_body(), " 2 5 11");
    }

    #[test]
    fn matrix_integration_reports_unported_dependencies() {
        let Err(SparseColumnError::MissingSparseMatrixPorts { dependencies }) =
            SparseColumn::full_sparse_matrix_integration()
        else {
            panic!("expected missing sparse matrix port error");
        };

        assert_eq!(dependencies, REQUIRED_INTEGRATION_PORT_BEADS);
    }
}
