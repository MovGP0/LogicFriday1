//! Native Rust port of `LogicSynthesis/sis/sparse/rows.c`.
//!
//! The C module stores each sparse-matrix row as a sorted doubly linked list of
//! `sm_element` entries keyed by `col_num`. This Rust port keeps the same
//! observable row-vector behavior with an owned, sorted, duplicate-free vector
//! of column numbers. C allocation/free-list routines are intentionally retired
//! in favor of Rust ownership.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SparseRow {
    row_num: i32,
    flag: i32,
    user_word: Option<String>,
    columns: Vec<i32>,
}

impl SparseRow {
    pub fn new() -> Self {
        Self {
            row_num: 0,
            flag: 0,
            user_word: None,
            columns: Vec::new(),
        }
    }

    pub fn with_row_num(row_num: i32) -> Self {
        Self {
            row_num,
            ..Self::new()
        }
    }

    pub fn row_num(&self) -> i32 {
        self.row_num
    }

    pub fn set_row_num(&mut self, row_num: i32) {
        self.row_num = row_num;
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
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    pub fn columns(&self) -> &[i32] {
        &self.columns
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = i32> + '_ {
        self.columns.iter().copied()
    }

    pub fn insert(&mut self, col: i32) -> RowElement {
        match self.columns.binary_search(&col) {
            Ok(index) => RowElement {
                col_num: self.columns[index],
                inserted: false,
            },
            Err(index) => {
                self.columns.insert(index, col);
                RowElement {
                    col_num: col,
                    inserted: true,
                }
            }
        }
    }

    pub fn remove(&mut self, col: i32) -> bool {
        let Ok(index) = self.columns.binary_search(&col) else {
            return false;
        };

        self.columns.remove(index);
        true
    }

    pub fn remove_element(&mut self, element: RowElement) -> bool {
        self.remove(element.col_num)
    }

    pub fn find(&self, col: i32) -> Option<RowElement> {
        self.columns
            .binary_search(&col)
            .ok()
            .map(|index| RowElement {
                col_num: self.columns[index],
                inserted: false,
            })
    }

    pub fn contains_row(&self, subset: &Self) -> bool {
        let mut superset = self.columns.iter();
        let mut next_super = superset.next();

        for col in &subset.columns {
            loop {
                match next_super {
                    Some(candidate) if candidate < col => next_super = superset.next(),
                    Some(candidate) if candidate == col => {
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

        while left < self.columns.len() && right < other.columns.len() {
            match self.columns[left].cmp(&other.columns[right]) {
                Ordering::Less => left += 1,
                Ordering::Greater => right += 1,
                Ordering::Equal => return true,
            }
        }

        false
    }

    pub fn sis_compare(&self, other: &Self) -> i32 {
        let mut left = self.columns.iter();
        let mut right = other.columns.iter();

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

        while left < self.columns.len() && right < other.columns.len() {
            match self.columns[left].cmp(&other.columns[right]) {
                Ordering::Less => left += 1,
                Ordering::Greater => right += 1,
                Ordering::Equal => {
                    result.columns.push(self.columns[left]);
                    left += 1;
                    right += 1;
                }
            }
        }

        result
    }

    pub fn sis_hash(&self, modulus: i32) -> Result<i32, SparseRowError> {
        if modulus <= 0 {
            return Err(SparseRowError::InvalidHashModulus(modulus));
        }

        Ok(self
            .columns
            .iter()
            .fold(0, |sum, col| (sum * 17 + col) % modulus))
    }

    pub fn print_body(&self) -> String {
        self.columns
            .iter()
            .map(|col| format!(" {col}"))
            .collect::<String>()
    }

    pub fn full_sparse_matrix_integration() -> Result<(), SparseRowError> {
        Err(SparseRowError::MissingSparseMatrixPorts)
    }
}

impl Default for SparseRow {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<i32> for SparseRow {
    fn from_iter<T: IntoIterator<Item = i32>>(iter: T) -> Self {
        let mut row = Self::new();
        for col in iter {
            row.insert(col);
        }
        row
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RowElement {
    col_num: i32,
    inserted: bool,
}

impl RowElement {
    pub fn col_num(self) -> i32 {
        self.col_num
    }

    pub fn was_inserted(self) -> bool {
        self.inserted
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SparseRowError {
    InvalidHashModulus(i32),
    MissingSparseMatrixPorts,
}

impl fmt::Display for SparseRowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHashModulus(modulus) => {
                write!(f, "invalid sparse-row hash modulus {modulus}")
            }
            Self::MissingSparseMatrixPorts => write!(
                f,
                "full sparse-row matrix integration requires unavailable native sparse matrix ports"
            ),
        }
    }
}

impl Error for SparseRowError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(columns: &[i32]) -> SparseRow {
        columns.iter().copied().collect()
    }

    #[test]
    fn starts_empty_with_sis_defaults() {
        let row = SparseRow::new();

        assert_eq!(row.row_num(), 0);
        assert_eq!(row.flag(), 0);
        assert_eq!(row.user_word(), None);
        assert!(row.is_empty());
        assert_eq!(row.print_body(), "");
    }

    #[test]
    fn insert_keeps_columns_sorted_and_unique() {
        let mut row = SparseRow::new();

        assert!(row.insert(7).was_inserted());
        assert!(row.insert(2).was_inserted());
        assert!(row.insert(5).was_inserted());
        assert!(!row.insert(5).was_inserted());

        assert_eq!(row.columns(), &[2, 5, 7]);
        assert_eq!(row.len(), 3);
    }

    #[test]
    fn find_and_remove_match_c_row_semantics() {
        let mut row = row(&[1, 3, 9]);

        assert_eq!(row.find(3).map(RowElement::col_num), Some(3));
        assert_eq!(row.find(4), None);
        assert!(row.remove(3));
        assert!(!row.remove(3));
        assert_eq!(row.columns(), &[1, 9]);

        let element = row.find(9).expect("column 9 is present");
        assert!(row.remove_element(element));
        assert_eq!(row.columns(), &[1]);
    }

    #[test]
    fn contains_tests_subset_relationship() {
        let superset = row(&[1, 2, 4, 9]);
        let subset = row(&[2, 9]);
        let missing = row(&[2, 8]);

        assert!(superset.contains_row(&subset));
        assert!(superset.contains_row(&SparseRow::new()));
        assert!(!subset.contains_row(&superset));
        assert!(!superset.contains_row(&missing));
    }

    #[test]
    fn intersects_uses_sorted_merge_walk() {
        assert!(row(&[1, 4, 8]).intersects(&row(&[2, 4, 9])));
        assert!(!row(&[1, 3]).intersects(&row(&[2, 4])));
        assert!(!SparseRow::new().intersects(&row(&[1])));
    }

    #[test]
    fn sis_compare_matches_rows_c_return_shape() {
        assert_eq!(row(&[1, 5]).sis_compare(&row(&[1, 7])), -2);
        assert_eq!(row(&[1, 7]).sis_compare(&row(&[1, 5])), 2);
        assert_eq!(row(&[1, 5]).sis_compare(&row(&[1, 5])), 0);
        assert_eq!(row(&[1, 5, 9]).sis_compare(&row(&[1, 5])), 1);
        assert_eq!(row(&[1, 5]).sis_compare(&row(&[1, 5, 9])), -1);
    }

    #[test]
    fn intersection_returns_new_row() {
        let result = row(&[1, 3, 5, 7]).intersection(&row(&[2, 3, 7, 11]));

        assert_eq!(result.columns(), &[3, 7]);
    }

    #[test]
    fn hash_uses_sis_recurrence() {
        let row = row(&[2, 5, 11]);

        assert_eq!(row.sis_hash(97), Ok((((2 * 17) + 5) * 17 + 11) % 97));
        assert_eq!(row.sis_hash(0), Err(SparseRowError::InvalidHashModulus(0)));
    }

    #[test]
    fn print_body_matches_c_spacing() {
        assert_eq!(row(&[2, 5, 11]).print_body(), " 2 5 11");
    }

    #[test]
    fn matrix_integration_reports_unavailable_native_ports() {
        assert_eq!(
            SparseRow::full_sparse_matrix_integration(),
            Err(SparseRowError::MissingSparseMatrixPorts)
        );
    }
}
