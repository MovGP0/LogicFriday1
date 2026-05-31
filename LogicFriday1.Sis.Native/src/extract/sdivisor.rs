//! Native Rust single-cube divisor selection for SIS extraction.
//!
//! The selector keeps the same incremental state as the SIS implementation:
//! columns are considered by descending literal count, pair coincidences are
//! cached in a max-heap, and stale heap entries are recomputed before use.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CubeLiteralMatrix {
    columns: BTreeMap<usize, BTreeSet<usize>>,
}

impl CubeLiteralMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, row: usize, column: usize) -> bool {
        self.columns.entry(column).or_default().insert(row)
    }

    pub fn remove(&mut self, row: usize, column: usize) -> bool {
        let Some(rows) = self.columns.get_mut(&column) else {
            return false;
        };

        let removed = rows.remove(&row);
        if rows.is_empty() {
            self.columns.remove(&column);
        }

        removed
    }

    pub fn remove_column(&mut self, column: usize) -> bool {
        self.columns.remove(&column).is_some()
    }

    pub fn row_count(&self) -> usize {
        self.columns
            .values()
            .flat_map(|rows| rows.iter().copied())
            .collect::<BTreeSet<_>>()
            .len()
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn column_length(&self, column: usize) -> Option<usize> {
        self.columns.get(&column).map(BTreeSet::len)
    }

    pub fn has_column(&self, column: usize) -> bool {
        self.columns.contains_key(&column)
    }

    pub fn from_columns(
        columns: impl IntoIterator<Item = (usize, impl IntoIterator<Item = usize>)>,
    ) -> Self {
        let mut matrix = Self::new();
        for (column, rows) in columns {
            for row in rows {
                matrix.insert(row, column);
            }
        }

        matrix
    }

    fn last_column(&self) -> Option<usize> {
        self.columns.keys().next_back().copied()
    }

    fn columns_after(&self, previous: Option<usize>) -> impl Iterator<Item = ColumnCell> + '_ {
        self.columns
            .iter()
            .filter(move |(column, _)| previous.is_none_or(|previous| **column > previous))
            .map(|(column, rows)| ColumnCell {
                number: *column,
                length: rows.len(),
            })
    }

    fn coincidence(&self, left: usize, right: usize) -> Option<usize> {
        let left_rows = self.columns.get(&left)?;
        let right_rows = self.columns.get(&right)?;
        Some(left_rows.intersection(right_rows).count())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SingleCubeDivisor {
    pub column1: usize,
    pub column2: usize,
    pub coincidence: usize,
}

impl SingleCubeDivisor {
    pub fn weight(&self) -> isize {
        self.coincidence as isize - 2
    }
}

#[derive(Clone, Debug, Default)]
pub struct SingleCubeDivisorSet {
    last_seen_column: Option<usize>,
    unconsidered_columns: Vec<ColumnCell>,
    considered_columns: Vec<ColumnCell>,
    heap: BinaryHeap<HeapEntry>,
    next_sequence: usize,
}

impl SingleCubeDivisorSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn extract(
        &mut self,
        matrix: &CubeLiteralMatrix,
        current_double_cube_weight: isize,
    ) -> Option<SingleCubeDivisor> {
        if matrix.row_count() == 0 {
            return None;
        }

        self.find_unconsidered_columns(matrix);
        self.find_divisor(matrix, current_double_cube_weight)
    }

    pub fn pending_candidate_count(&self) -> usize {
        self.heap.len()
    }

    pub fn considered_column_count(&self) -> usize {
        self.considered_columns.len()
    }

    fn find_unconsidered_columns(&mut self, matrix: &CubeLiteralMatrix) {
        if !self.unconsidered_columns.is_empty() {
            self.unconsidered_columns
                .retain(|column| matrix.has_column(column.number));
        } else if self.last_seen_column == matrix.last_column() {
            return;
        }

        if self.last_seen_column == matrix.last_column() {
            self.sort_unconsidered_columns();
            return;
        }

        self.unconsidered_columns
            .extend(matrix.columns_after(self.last_seen_column));
        self.last_seen_column = matrix.last_column();
        self.sort_unconsidered_columns();
    }

    fn find_divisor(
        &mut self,
        matrix: &CubeLiteralMatrix,
        current_double_cube_weight: isize,
    ) -> Option<SingleCubeDivisor> {
        loop {
            let mut no_iteration = false;
            let k = if let Some(column) = self.unconsidered_columns.last().copied() {
                let k = column.length;
                if Self::weight_for_coincidence(k) > current_double_cube_weight {
                    let columns = self.extract_columns_with_length(k);
                    self.compute_new_pair_coincidences(matrix, &columns);
                    self.compute_existing_pair_coincidences(matrix, &columns);
                    self.considered_columns.extend(columns);
                } else {
                    no_iteration = true;
                    let max_key = self.heap.peek().map(|entry| entry.candidate.coincidence);
                    if max_key.is_none_or(|key| {
                        Self::weight_for_coincidence(key) <= current_double_cube_weight
                    }) {
                        return None;
                    }
                }

                k
            } else {
                no_iteration = true;
                0
            };

            loop {
                let Some(mut entry) = self.heap.pop() else {
                    if no_iteration {
                        return None;
                    }

                    break;
                };

                let candidate = &mut entry.candidate;
                let Some(coincidence) = matrix.coincidence(candidate.column1, candidate.column2)
                else {
                    continue;
                };

                if coincidence >= candidate.coincidence {
                    candidate.coincidence = coincidence;
                    if coincidence >= k
                        && Self::weight_for_coincidence(coincidence) > current_double_cube_weight
                    {
                        return Some(*candidate);
                    }

                    if coincidence > 2 {
                        self.push_candidate(*candidate);
                    }

                    break;
                }

                if coincidence > 2 {
                    candidate.coincidence = coincidence;
                    self.push_candidate(*candidate);
                }
            }

            if no_iteration {
                return None;
            }
        }
    }

    fn extract_columns_with_length(&mut self, length: usize) -> Vec<ColumnCell> {
        let mut columns = Vec::new();
        while self
            .unconsidered_columns
            .last()
            .is_some_and(|column| column.length == length)
        {
            let column = self
                .unconsidered_columns
                .pop()
                .expect("last column was already checked");
            columns.push(column);
        }

        columns
    }

    fn compute_new_pair_coincidences(
        &mut self,
        matrix: &CubeLiteralMatrix,
        columns: &[ColumnCell],
    ) {
        if columns.len() <= 1 {
            return;
        }

        for left_index in 0..columns.len() - 1 {
            for right in &columns[left_index + 1..] {
                self.push_coincidence(matrix, columns[left_index].number, right.number);
            }
        }
    }

    fn compute_existing_pair_coincidences(
        &mut self,
        matrix: &CubeLiteralMatrix,
        columns: &[ColumnCell],
    ) {
        if columns.is_empty() || self.considered_columns.is_empty() {
            return;
        }

        self.considered_columns
            .retain(|column| matrix.has_column(column.number));
        let considered_columns = self.considered_columns.clone();
        for column in columns {
            for considered in &considered_columns {
                self.push_coincidence(matrix, column.number, considered.number);
            }
        }
    }

    fn push_coincidence(&mut self, matrix: &CubeLiteralMatrix, left: usize, right: usize) {
        let Some(coincidence) = matrix.coincidence(left, right) else {
            return;
        };

        if coincidence > 2 {
            self.push_candidate(SingleCubeDivisor {
                column1: left,
                column2: right,
                coincidence,
            });
        }
    }

    fn push_candidate(&mut self, candidate: SingleCubeDivisor) {
        let sequence = self.next_sequence;
        self.next_sequence += 1;
        self.heap.push(HeapEntry {
            candidate,
            sequence,
        });
    }

    fn sort_unconsidered_columns(&mut self) {
        self.unconsidered_columns.sort_unstable_by(|left, right| {
            left.length
                .cmp(&right.length)
                .then_with(|| right.number.cmp(&left.number))
        });
    }

    fn weight_for_coincidence(coincidence: usize) -> isize {
        coincidence as isize - 2
    }
}

pub fn extract_single_cube_divisor(
    matrix: &CubeLiteralMatrix,
    state: &mut SingleCubeDivisorSet,
    current_double_cube_weight: isize,
) -> Option<SingleCubeDivisor> {
    state.extract(matrix, current_double_cube_weight)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ColumnCell {
    number: usize,
    length: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HeapEntry {
    candidate: SingleCubeDivisor,
    sequence: usize,
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.candidate
            .coincidence
            .cmp(&other.candidate.coincidence)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(columns: &[(usize, &[usize])]) -> CubeLiteralMatrix {
        let mut matrix = CubeLiteralMatrix::new();
        for (column, rows) in columns {
            for row in *rows {
                matrix.insert(*row, *column);
            }
        }

        matrix
    }

    #[test]
    fn empty_matrix_has_no_single_cube_divisor() {
        let matrix = CubeLiteralMatrix::new();
        let mut state = SingleCubeDivisorSet::new();

        assert_eq!(extract_single_cube_divisor(&matrix, &mut state, 0), None);
    }

    #[test]
    fn selects_pair_with_coincidence_above_two_and_weight_threshold() {
        let matrix = matrix(&[(0, &[0, 1, 2, 3]), (1, &[1, 2, 3, 4]), (2, &[7, 8, 9, 10])]);
        let mut state = SingleCubeDivisorSet::new();

        let divisor = extract_single_cube_divisor(&matrix, &mut state, 0).unwrap();

        assert_eq!(
            divisor,
            SingleCubeDivisor {
                column1: 0,
                column2: 1,
                coincidence: 3,
            }
        );
        assert_eq!(divisor.weight(), 1);
    }

    #[test]
    fn rejects_candidates_that_cannot_beat_double_cube_weight() {
        let matrix = matrix(&[(0, &[0, 1, 2, 3]), (1, &[1, 2, 3, 4])]);
        let mut state = SingleCubeDivisorSet::new();

        assert_eq!(extract_single_cube_divisor(&matrix, &mut state, 1), None);
        assert_eq!(state.pending_candidate_count(), 1);
    }

    #[test]
    fn persists_considered_columns_across_incremental_calls() {
        let mut matrix = matrix(&[(0, &[0, 1, 2, 3]), (1, &[4, 5, 6, 7])]);
        let mut state = SingleCubeDivisorSet::new();

        assert_eq!(extract_single_cube_divisor(&matrix, &mut state, 0), None);
        assert_eq!(state.considered_column_count(), 2);

        matrix.insert(1, 2);
        matrix.insert(2, 2);
        matrix.insert(3, 2);
        matrix.insert(8, 2);

        let divisor = extract_single_cube_divisor(&matrix, &mut state, 0).unwrap();

        assert_eq!(
            divisor,
            SingleCubeDivisor {
                column1: 2,
                column2: 0,
                coincidence: 3,
            }
        );
    }

    #[test]
    fn recomputes_heap_entries_and_discards_stale_columns() {
        let mut matrix = matrix(&[(0, &[0, 1, 2, 3]), (1, &[0, 1, 2, 4])]);
        let mut state = SingleCubeDivisorSet::new();

        assert_eq!(extract_single_cube_divisor(&matrix, &mut state, 1), None);
        assert_eq!(state.pending_candidate_count(), 1);

        matrix.remove_column(1);

        assert_eq!(extract_single_cube_divisor(&matrix, &mut state, 0), None);
        assert_eq!(state.pending_candidate_count(), 0);
    }

    #[test]
    fn matrix_removes_empty_column_after_last_literal_is_deleted() {
        let mut matrix = matrix(&[(4, &[2])]);

        assert_eq!(matrix.column_length(4), Some(1));
        assert!(matrix.remove(2, 4));
        assert_eq!(matrix.column_length(4), None);
        assert_eq!(matrix.row_count(), 0);
    }
}
