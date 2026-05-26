//! Native Rust port of `sis/sparse/matrix.c`.
//!
//! The original SIS sparse matrix stores one linked `sm_element` in both a row
//! list and a column list. This port keeps the same sorted coordinate behavior
//! with owned Rust collections instead of raw pointers and allocator freelists.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SparseElement {
    pub row: usize,
    pub col: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SparseVector {
    index: usize,
    elements: Vec<usize>,
}

impl SparseVector {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn elements(&self) -> &[usize] {
        &self.elements
    }

    pub fn contains(&self, index: usize) -> bool {
        self.elements.binary_search(&index).is_ok()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SparseMatrix {
    rows: BTreeMap<usize, BTreeSet<usize>>,
    cols: BTreeMap<usize, BTreeSet<usize>>,
    rows_size: usize,
    cols_size: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SparseParseError {
    MissingMatrixSize,
    InvalidMatrixSize,
    InvalidPair { line: usize },
    InvalidCompressedRow { row: usize },
    InvalidCompressedWord { row: usize, block: usize },
}

impl fmt::Display for SparseParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMatrixSize => write!(f, "missing compressed matrix size"),
            Self::InvalidMatrixSize => write!(f, "invalid compressed matrix size"),
            Self::InvalidPair { line } => write!(f, "invalid row/column pair on line {line}"),
            Self::InvalidCompressedRow { row } => {
                write!(f, "invalid compressed row header for row {row}")
            }
            Self::InvalidCompressedWord { row, block } => {
                write!(f, "invalid compressed word for row {row}, block {block}")
            }
        }
    }
}

impl std::error::Error for SparseParseError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingSparseDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const UNPORTED_VECTOR_DEPENDENCIES: &[MissingSparseDependency] = &[
    MissingSparseDependency {
        bead_id: "LogicFriday1-8j8.2.6.458",
        source_file: "LogicSynthesis/sis/sparse/rows.c",
        reason: "standalone row-vector allocation, comparison, intersection, hashing, and printing APIs",
    },
    MissingSparseDependency {
        bead_id: "LogicFriday1-8j8.2.6.456",
        source_file: "LogicSynthesis/sis/sparse/cols.c",
        reason: "standalone column-vector allocation, comparison, intersection, hashing, and printing APIs",
    },
];

impl Default for SparseMatrix {
    fn default() -> Self {
        Self::new()
    }
}

impl SparseMatrix {
    pub fn new() -> Self {
        Self {
            rows: BTreeMap::new(),
            cols: BTreeMap::new(),
            rows_size: 0,
            cols_size: 0,
        }
    }

    pub fn with_size(row: usize, col: usize) -> Self {
        let mut matrix = Self::new();
        matrix.resize(row, col);
        matrix
    }

    pub fn resize(&mut self, row: usize, col: usize) {
        if row >= self.rows_size {
            self.rows_size = (self.rows_size * 2).max(row + 1);
        }

        if col >= self.cols_size {
            self.cols_size = (self.cols_size * 2).max(col + 1);
        }
    }

    pub fn rows_size(&self) -> usize {
        self.rows_size
    }

    pub fn cols_size(&self) -> usize {
        self.cols_size
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize {
        self.cols.len()
    }

    pub fn element_count(&self) -> usize {
        self.rows.values().map(BTreeSet::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn insert(&mut self, row: usize, col: usize) -> bool {
        if row >= self.rows_size || col >= self.cols_size {
            self.resize(row, col);
        }

        let inserted = self.rows.entry(row).or_default().insert(col);
        if inserted {
            self.cols.entry(col).or_default().insert(row);
        }
        inserted
    }

    pub fn find(&self, row: usize, col: usize) -> Option<SparseElement> {
        self.contains(row, col)
            .then_some(SparseElement { row, col })
    }

    pub fn contains(&self, row: usize, col: usize) -> bool {
        match (self.rows.get(&row), self.cols.get(&col)) {
            (Some(row_set), Some(col_set)) if row_set.len() < col_set.len() => {
                row_set.contains(&col)
            }
            (_, Some(col_set)) => col_set.contains(&row),
            _ => false,
        }
    }

    pub fn remove(&mut self, row: usize, col: usize) -> bool {
        if !self.contains(row, col) {
            return false;
        }

        self.remove_existing(row, col);
        true
    }

    pub fn remove_element(&mut self, element: SparseElement) -> bool {
        self.remove(element.row, element.col)
    }

    pub fn delete_row(&mut self, row: usize) -> bool {
        let Some(cols) = self.rows.remove(&row) else {
            return false;
        };

        for col in cols {
            self.remove_row_from_col(row, col);
        }

        true
    }

    pub fn delete_col(&mut self, col: usize) -> bool {
        let Some(rows) = self.cols.remove(&col) else {
            return false;
        };

        for row in rows {
            self.remove_col_from_row(row, col);
        }

        true
    }

    pub fn row(&self, row: usize) -> Option<SparseVector> {
        self.rows.get(&row).map(|cols| SparseVector {
            index: row,
            elements: cols.iter().copied().collect(),
        })
    }

    pub fn col(&self, col: usize) -> Option<SparseVector> {
        self.cols.get(&col).map(|rows| SparseVector {
            index: col,
            elements: rows.iter().copied().collect(),
        })
    }

    pub fn rows(&self) -> impl Iterator<Item = SparseVector> + '_ {
        self.rows.iter().map(|(row, cols)| SparseVector {
            index: *row,
            elements: cols.iter().copied().collect(),
        })
    }

    pub fn cols(&self) -> impl Iterator<Item = SparseVector> + '_ {
        self.cols.iter().map(|(col, rows)| SparseVector {
            index: *col,
            elements: rows.iter().copied().collect(),
        })
    }

    pub fn elements(&self) -> impl Iterator<Item = SparseElement> + '_ {
        self.rows.iter().flat_map(|(row, cols)| {
            cols.iter().map(|col| SparseElement {
                row: *row,
                col: *col,
            })
        })
    }

    pub fn copy_row_from(&mut self, dest_row: usize, source: &SparseVector) {
        for col in source.elements() {
            self.insert(dest_row, *col);
        }
    }

    pub fn copy_col_from(&mut self, dest_col: usize, source: &SparseVector) {
        for row in source.elements() {
            self.insert(*row, dest_col);
        }
    }

    pub fn longest_row(&self) -> Option<SparseVector> {
        self.rows().fold(None, |best, row| match best {
            Some(best) if best.len() >= row.len() => Some(best),
            _ => Some(row),
        })
    }

    pub fn longest_col(&self) -> Option<SparseVector> {
        self.cols().fold(None, |best, col| match best {
            Some(best) if best.len() >= col.len() => Some(best),
            _ => Some(col),
        })
    }

    pub fn from_pairs(input: &str) -> Result<Self, SparseParseError> {
        let mut matrix = Self::new();

        for (line_index, line) in input.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut parts = line.split_whitespace();
            let row = parts
                .next()
                .and_then(|part| part.parse::<usize>().ok())
                .ok_or(SparseParseError::InvalidPair {
                    line: line_index + 1,
                })?;
            let col = parts
                .next()
                .and_then(|part| part.parse::<usize>().ok())
                .ok_or(SparseParseError::InvalidPair {
                    line: line_index + 1,
                })?;

            if parts.next().is_some() {
                return Err(SparseParseError::InvalidPair {
                    line: line_index + 1,
                });
            }

            matrix.insert(row, col);
        }

        Ok(matrix)
    }

    pub fn from_compressed(input: &str) -> Result<Self, SparseParseError> {
        let mut tokens = input.split_whitespace();
        let nrows = tokens
            .next()
            .ok_or(SparseParseError::MissingMatrixSize)?
            .parse::<usize>()
            .map_err(|_| SparseParseError::InvalidMatrixSize)?;
        let ncols = tokens
            .next()
            .ok_or(SparseParseError::MissingMatrixSize)?
            .parse::<usize>()
            .map_err(|_| SparseParseError::InvalidMatrixSize)?;

        let mut matrix = Self::with_size(nrows, ncols);
        for row in 0..nrows {
            let _row_header = tokens
                .next()
                .ok_or(SparseParseError::InvalidCompressedRow { row })?
                .parse::<u64>()
                .map_err(|_| SparseParseError::InvalidCompressedRow { row })?;

            for block in (0..ncols).step_by(32).enumerate() {
                let word = tokens
                    .next()
                    .ok_or(SparseParseError::InvalidCompressedWord {
                        row,
                        block: block.0,
                    })
                    .and_then(|token| {
                        u64::from_str_radix(token, 16).map_err(|_| {
                            SparseParseError::InvalidCompressedWord {
                                row,
                                block: block.0,
                            }
                        })
                    })?;

                let mut bits = word;
                let mut col = block.1;
                while bits != 0 {
                    if bits & 1 != 0 {
                        matrix.insert(row, col);
                    }
                    bits >>= 1;
                    col += 1;
                }
            }
        }

        Ok(matrix)
    }

    pub fn write_pairs(&self) -> String {
        let mut output = String::new();
        for element in self.elements() {
            writeln!(&mut output, "{} {}", element.row, element.col)
                .expect("writing to a String should not fail");
        }
        output
    }

    pub fn print_layout(&self) -> String {
        let mut output = String::new();
        self.write_layout(&mut output)
            .expect("writing to a String should not fail");
        output
    }

    pub fn write_layout(&self, writer: &mut impl Write) -> fmt::Result {
        let Some(last_col) = self.cols.keys().next_back().copied() else {
            return Ok(());
        };

        if last_col >= 100 {
            writer.write_str("    ")?;
            for col in self.cols.keys() {
                write!(writer, "{}", (col / 100) % 10)?;
            }
            writer.write_char('\n')?;
        }

        if last_col >= 10 {
            writer.write_str("    ")?;
            for col in self.cols.keys() {
                write!(writer, "{}", (col / 10) % 10)?;
            }
            writer.write_char('\n')?;
        }

        writer.write_str("    ")?;
        for col in self.cols.keys() {
            write!(writer, "{}", col % 10)?;
        }
        writer.write_char('\n')?;

        writer.write_str("    ")?;
        for _ in self.cols.keys() {
            writer.write_char('-')?;
        }
        writer.write_char('\n')?;

        for (row, cols) in &self.rows {
            write!(writer, "{row:3}:")?;
            for col in self.cols.keys() {
                writer.write_char(if cols.contains(col) { '1' } else { '.' })?;
            }
            writer.write_char('\n')?;
        }

        Ok(())
    }

    pub fn dump(&self, label: &str, max_rows: usize) -> String {
        let mut output = format!(
            "{label} {} rows by {} cols\n",
            self.row_count(),
            self.col_count()
        );
        if self.row_count() < max_rows {
            output.push_str(&self.print_layout());
        }
        output
    }

    fn remove_existing(&mut self, row: usize, col: usize) {
        self.remove_col_from_row(row, col);
        self.remove_row_from_col(row, col);
    }

    fn remove_col_from_row(&mut self, row: usize, col: usize) {
        let should_remove_row = if let Some(cols) = self.rows.get_mut(&row) {
            cols.remove(&col);
            cols.is_empty()
        } else {
            false
        };

        if should_remove_row {
            self.rows.remove(&row);
        }
    }

    fn remove_row_from_col(&mut self, row: usize, col: usize) {
        let should_remove_col = if let Some(rows) = self.cols.get_mut(&col) {
            rows.remove(&row);
            rows.is_empty()
        } else {
            false
        };

        if should_remove_col {
            self.cols.remove(&col);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty_and_resizes_like_sparse_matrix() {
        let mut matrix = SparseMatrix::new();

        assert_eq!(matrix.row_count(), 0);
        assert_eq!(matrix.col_count(), 0);
        assert_eq!(matrix.rows_size(), 0);
        assert_eq!(matrix.cols_size(), 0);

        matrix.resize(3, 5);

        assert_eq!(matrix.rows_size(), 4);
        assert_eq!(matrix.cols_size(), 6);
        assert!(matrix.is_empty());
    }

    #[test]
    fn inserts_find_and_ignores_duplicate_coordinates() {
        let mut matrix = SparseMatrix::new();

        assert!(matrix.insert(4, 2));
        assert!(!matrix.insert(4, 2));
        assert!(matrix.insert(1, 7));

        assert_eq!(matrix.element_count(), 2);
        assert_eq!(matrix.find(4, 2), Some(SparseElement { row: 4, col: 2 }));
        assert!(matrix.contains(1, 7));
        assert!(!matrix.contains(7, 1));
        assert_eq!(
            matrix.elements().collect::<Vec<_>>(),
            vec![
                SparseElement { row: 1, col: 7 },
                SparseElement { row: 4, col: 2 }
            ]
        );
    }

    #[test]
    fn removal_discards_empty_row_and_column_headers() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(0, 0);
        matrix.insert(0, 1);
        matrix.insert(2, 1);

        assert!(matrix.remove(0, 0));
        assert_eq!(matrix.row(0).unwrap().elements(), &[1]);
        assert!(matrix.col(0).is_none());

        assert!(matrix.remove(0, 1));
        assert!(matrix.row(0).is_none());
        assert_eq!(matrix.col(1).unwrap().elements(), &[2]);
        assert!(!matrix.remove(0, 1));
    }

    #[test]
    fn deletes_rows_and_columns_bidirectionally() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(0, 0);
        matrix.insert(0, 2);
        matrix.insert(1, 2);
        matrix.insert(3, 3);

        assert!(matrix.delete_row(0));
        assert!(!matrix.contains(0, 0));
        assert!(!matrix.contains(0, 2));
        assert_eq!(matrix.col(2).unwrap().elements(), &[1]);

        assert!(matrix.delete_col(2));
        assert!(matrix.row(1).is_none());
        assert_eq!(matrix.element_count(), 1);
        assert!(!matrix.delete_col(2));
    }

    #[test]
    fn copies_rows_and_columns_from_snapshots() {
        let mut source = SparseMatrix::new();
        source.insert(1, 3);
        source.insert(1, 5);
        source.insert(2, 5);

        let row = source.row(1).unwrap();
        let col = source.col(5).unwrap();
        let mut dest = SparseMatrix::new();
        dest.copy_row_from(7, &row);
        dest.copy_col_from(9, &col);

        assert_eq!(dest.row(7).unwrap().elements(), &[3, 5]);
        assert_eq!(dest.col(9).unwrap().elements(), &[1, 2]);
    }

    #[test]
    fn reports_longest_row_and_column_with_first_c_tie_behavior() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(4, 1);
        matrix.insert(4, 2);
        matrix.insert(2, 5);
        matrix.insert(2, 6);
        matrix.insert(1, 2);
        matrix.insert(3, 2);

        assert_eq!(matrix.longest_row().unwrap().index(), 2);
        assert_eq!(matrix.longest_col().unwrap().index(), 2);
    }

    #[test]
    fn reads_and_writes_pair_format_in_sorted_order() {
        let matrix = SparseMatrix::from_pairs("3 2\n1 5\n3 2\n").unwrap();

        assert_eq!(matrix.element_count(), 2);
        assert_eq!(matrix.write_pairs(), "1 5\n3 2\n");
    }

    #[test]
    fn reads_compressed_format() {
        let matrix = SparseMatrix::from_compressed("2 35 0 80000001 4 0 2 0").unwrap();

        assert!(matrix.contains(0, 0));
        assert!(matrix.contains(0, 31));
        assert!(matrix.contains(0, 34));
        assert!(matrix.contains(1, 1));
        assert_eq!(matrix.element_count(), 4);
    }

    #[test]
    fn prints_sparse_layout_using_active_columns_only() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(2, 0);
        matrix.insert(2, 12);
        matrix.insert(4, 12);

        assert_eq!(
            matrix.print_layout(),
            "    01\n    02\n    --\n  2:11\n  4:.1\n"
        );
    }
}
