//! Native Rust port of `LogicSynthesis/sis/extract/pingpong.c`.
//!
//! The original routine coordinates row- and column-seeded greedy rectangle
//! searches over a weighted sparse matrix. This port exposes the behavior as
//! owned Rust data structures and functions instead of SIS pointers or C ABI
//! entry points.

use std::collections::{BTreeMap, BTreeSet};

pub type MatrixIndex = usize;
pub type MatrixCost = i32;
pub type MatrixValue = i32;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PingPongMatrix {
    rows: BTreeMap<MatrixIndex, BTreeMap<MatrixIndex, MatrixValue>>,
    cols: BTreeMap<MatrixIndex, BTreeMap<MatrixIndex, MatrixValue>>,
}

impl PingPongMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &mut self,
        row: MatrixIndex,
        col: MatrixIndex,
        value: MatrixValue,
    ) -> Option<MatrixValue> {
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
        self.rows.is_empty()
    }

    pub fn row(&self, row: MatrixIndex) -> Option<&BTreeMap<MatrixIndex, MatrixValue>> {
        self.rows.get(&row)
    }

    pub fn col(&self, col: MatrixIndex) -> Option<&BTreeMap<MatrixIndex, MatrixValue>> {
        self.cols.get(&col)
    }

    pub fn rows(&self) -> impl Iterator<Item = (MatrixIndex, &BTreeMap<MatrixIndex, MatrixValue>)> {
        self.rows.iter().map(|(row, cols)| (*row, cols))
    }

    pub fn cols(&self) -> impl Iterator<Item = (MatrixIndex, &BTreeMap<MatrixIndex, MatrixValue>)> {
        self.cols.iter().map(|(col, rows)| (*col, rows))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CostTable {
    costs: Vec<MatrixCost>,
}

impl CostTable {
    pub fn new(costs: Vec<MatrixCost>) -> Self {
        Self { costs }
    }

    pub fn cost(&self, index: MatrixIndex) -> MatrixCost {
        self.costs.get(index).copied().unwrap_or_default()
    }
}

impl From<Vec<MatrixCost>> for CostTable {
    fn from(costs: Vec<MatrixCost>) -> Self {
        Self::new(costs)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Rectangle {
    rows: BTreeSet<MatrixIndex>,
    cols: BTreeSet<MatrixIndex>,
    value: MatrixValue,
}

impl Rectangle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rows(&self) -> &BTreeSet<MatrixIndex> {
        &self.rows
    }

    pub fn cols(&self) -> &BTreeSet<MatrixIndex> {
        &self.cols
    }

    pub fn value(&self) -> MatrixValue {
        self.value
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize {
        self.cols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() || self.cols.is_empty()
    }
}

pub fn ping_pong(matrix: &PingPongMatrix, row_cost: &CostTable, col_cost: &CostTable) -> Rectangle {
    let mut best_rect = ping_pong_onepass(matrix, row_cost, col_cost);
    if !best_rect.is_empty() {
        best_rect = ping_pong_improve(matrix, row_cost, col_cost, best_rect);
    }

    best_rect
}

fn row_value(
    row: &BTreeMap<MatrixIndex, MatrixValue>,
    row_num: MatrixIndex,
    row_cost: &CostTable,
) -> MatrixValue {
    row.values().sum::<MatrixValue>() - row_cost.cost(row_num)
}

fn choose_row_seed(matrix: &PingPongMatrix, row_cost: &CostTable) -> Option<MatrixIndex> {
    matrix
        .rows()
        .fold(None, |best, (row_num, row)| {
            let value = row_value(row, row_num, row_cost);
            match best {
                Some((_, max_value)) if max_value >= value => best,
                _ => Some((row_num, value)),
            }
        })
        .map(|(row_num, _)| row_num)
}

fn choose_next_row_seed(
    matrix: &PingPongMatrix,
    row_cost: &CostTable,
    rect: &Rectangle,
) -> Option<MatrixIndex> {
    rect.rows
        .iter()
        .fold(None, |best, row_num| {
            let Some(row) = matrix.row(*row_num) else {
                return best;
            };

            let value = row_value(row, *row_num, row_cost);
            match best {
                Some((_, max_value)) if max_value >= value => best,
                _ => Some((*row_num, value)),
            }
        })
        .map(|(row_num, _)| row_num)
}

fn col_value(
    col: &BTreeMap<MatrixIndex, MatrixValue>,
    col_num: MatrixIndex,
    col_cost: &CostTable,
) -> MatrixValue {
    col.values().sum::<MatrixValue>() - col_cost.cost(col_num)
}

fn choose_col_seed(matrix: &PingPongMatrix, col_cost: &CostTable) -> Option<MatrixIndex> {
    matrix
        .cols()
        .fold(None, |best, (col_num, col)| {
            let value = col_value(col, col_num, col_cost);
            match best {
                Some((_, max_value)) if max_value >= value => best,
                _ => Some((col_num, value)),
            }
        })
        .map(|(col_num, _)| col_num)
}

fn choose_next_col_seed(
    matrix: &PingPongMatrix,
    col_cost: &CostTable,
    rect: &Rectangle,
) -> Option<MatrixIndex> {
    rect.cols
        .iter()
        .fold(None, |best, col_num| {
            let Some(col) = matrix.col(*col_num) else {
                return best;
            };

            let value = col_value(col, *col_num, col_cost);
            match best {
                Some((_, max_value)) if max_value >= value => best,
                _ => Some((*col_num, value)),
            }
        })
        .map(|(col_num, _)| col_num)
}

fn ping_pong_onepass(
    matrix: &PingPongMatrix,
    row_cost: &CostTable,
    col_cost: &CostTable,
) -> Rectangle {
    let Some(row_seed) = choose_row_seed(matrix, row_cost) else {
        return Rectangle::new();
    };

    let rect1 = greedy_row(matrix, row_cost, col_cost, row_seed);
    let Some(col_seed) = choose_col_seed(matrix, col_cost) else {
        return rect1;
    };

    let rect2 = greedy_col(matrix, col_cost, row_cost, col_seed);
    if rect1.value > rect2.value {
        rect1
    } else {
        rect2
    }
}

fn ping_pong_improve(
    matrix: &PingPongMatrix,
    row_cost: &CostTable,
    col_cost: &CostTable,
    mut best_rect: Rectangle,
) -> Rectangle {
    loop {
        let Some(best_row) = choose_next_row_seed(matrix, row_cost, &best_rect) else {
            return best_rect;
        };

        let rect1 = greedy_row(matrix, row_cost, col_cost, best_row);
        if rect1.value < 1 {
            break;
        }

        let Some(best_col) = choose_next_col_seed(matrix, col_cost, &rect1) else {
            return best_rect;
        };

        let rect2 = greedy_col(matrix, col_cost, row_cost, best_col);
        let got_new_best = update_best(rect1, rect2, &mut best_rect);
        if !got_new_best {
            break;
        }
    }

    best_rect
}

fn update_best(rect1: Rectangle, rect2: Rectangle, best_rect: &mut Rectangle) -> bool {
    if rect1.value >= rect2.value {
        if rect1.value > best_rect.value {
            *best_rect = rect1;
        }

        false
    } else if rect2.value > best_rect.value {
        *best_rect = rect2;
        true
    } else {
        false
    }
}

fn greedy_row(
    matrix: &PingPongMatrix,
    row_cost: &CostTable,
    col_cost: &CostTable,
    seed: MatrixIndex,
) -> Rectangle {
    let mut best_rect = Rectangle::new();
    let Some(seed_row) = matrix.row(seed) else {
        return best_rect;
    };

    if matrix.row_count() == 0 {
        return best_rect;
    }

    let mut rect = Rectangle::new();
    rect.rows.insert(seed);
    rect.cols.extend(seed_row.keys().copied());

    let mut col_values = BTreeMap::new();
    for (col, value) in seed_row {
        col_values.insert(*col, value - col_cost.cost(*col));
    }

    let mut total_row_cost = row_cost.cost(seed);
    rect.value = -total_row_cost
        + rect
            .cols
            .iter()
            .map(|col| col_values.get(col).copied().unwrap_or_default())
            .sum::<MatrixValue>();

    let mut next_row = find_max_row_intersection(matrix, row_cost, &rect, &col_values);
    while let Some(row_num) = next_row {
        if rect.cols.len() <= 1 {
            break;
        }

        let row = matrix
            .row(row_num)
            .expect("selected row must still be present in matrix");
        rect.rows.insert(row_num);
        total_row_cost += row_cost.cost(row_num);
        rect.cols = rect
            .cols
            .intersection(&row.keys().copied().collect())
            .copied()
            .collect();

        for (col, value) in row {
            *col_values.entry(*col).or_default() += value;
        }

        rect.value = -total_row_cost
            + rect
                .cols
                .iter()
                .map(|col| col_values.get(col).copied().unwrap_or_default())
                .sum::<MatrixValue>();

        if rect.value > best_rect.value {
            best_rect = rect.clone();
        }

        next_row = find_max_row_intersection(matrix, row_cost, &rect, &col_values);
    }

    best_rect
}

fn find_max_row_intersection(
    matrix: &PingPongMatrix,
    row_cost: &CostTable,
    rect: &Rectangle,
    col_values: &BTreeMap<MatrixIndex, MatrixValue>,
) -> Option<MatrixIndex> {
    if matrix.row_count() == 0 {
        return None;
    }

    let mut row_values = BTreeMap::new();
    for col in &rect.cols {
        let Some(matrix_col) = matrix.col(*col) else {
            continue;
        };

        for row in matrix_col.keys() {
            row_values.insert(*row, -row_cost.cost(*row));
        }
    }

    for col in &rect.cols {
        let Some(matrix_col) = matrix.col(*col) else {
            continue;
        };

        for (row, value) in matrix_col {
            *row_values.entry(*row).or_default() +=
                value + col_values.get(col).copied().unwrap_or_default();
        }
    }

    let mut best_row = None;
    let mut max_value = 0;
    for col in &rect.cols {
        let Some(matrix_col) = matrix.col(*col) else {
            continue;
        };

        for row in matrix_col.keys() {
            let value = row_values.get(row).copied().unwrap_or_default();
            if value > max_value && !rect.rows.contains(row) {
                max_value = value;
                best_row = Some(*row);
            }
        }
    }

    best_row
}

fn greedy_col(
    matrix: &PingPongMatrix,
    col_cost: &CostTable,
    row_cost: &CostTable,
    seed: MatrixIndex,
) -> Rectangle {
    let mut best_rect = Rectangle::new();
    let Some(seed_col) = matrix.col(seed) else {
        return best_rect;
    };

    if matrix.col_count() == 0 {
        return best_rect;
    }

    let mut rect = Rectangle::new();
    rect.cols.insert(seed);
    rect.rows.extend(seed_col.keys().copied());

    let mut row_values = BTreeMap::new();
    for (row, value) in seed_col {
        row_values.insert(*row, value - row_cost.cost(*row));
    }

    let mut total_col_cost = col_cost.cost(seed);
    rect.value = -total_col_cost
        + rect
            .rows
            .iter()
            .map(|row| row_values.get(row).copied().unwrap_or_default())
            .sum::<MatrixValue>();

    let mut next_col = find_max_col_intersection(matrix, col_cost, &rect, &row_values);
    while let Some(col_num) = next_col {
        if rect.rows.len() <= 1 {
            break;
        }

        let col = matrix
            .col(col_num)
            .expect("selected column must still be present in matrix");
        rect.cols.insert(col_num);
        total_col_cost += col_cost.cost(col_num);
        rect.rows = rect
            .rows
            .intersection(&col.keys().copied().collect())
            .copied()
            .collect();

        for (row, value) in col {
            *row_values.entry(*row).or_default() += value;
        }

        rect.value = -total_col_cost
            + rect
                .rows
                .iter()
                .map(|row| row_values.get(row).copied().unwrap_or_default())
                .sum::<MatrixValue>();

        if rect.value > best_rect.value {
            best_rect = rect.clone();
        }

        next_col = find_max_col_intersection(matrix, col_cost, &rect, &row_values);
    }

    best_rect
}

fn find_max_col_intersection(
    matrix: &PingPongMatrix,
    col_cost: &CostTable,
    rect: &Rectangle,
    row_values: &BTreeMap<MatrixIndex, MatrixValue>,
) -> Option<MatrixIndex> {
    if matrix.col_count() == 0 {
        return None;
    }

    let mut col_values = BTreeMap::new();
    for row in &rect.rows {
        let Some(matrix_row) = matrix.row(*row) else {
            continue;
        };

        for col in matrix_row.keys() {
            col_values.insert(*col, -col_cost.cost(*col));
        }
    }

    for row in &rect.rows {
        let Some(matrix_row) = matrix.row(*row) else {
            continue;
        };

        for (col, value) in matrix_row {
            *col_values.entry(*col).or_default() +=
                value + row_values.get(row).copied().unwrap_or_default();
        }
    }

    let mut best_col = None;
    let mut max_value = 0;
    for row in &rect.rows {
        let Some(matrix_row) = matrix.row(*row) else {
            continue;
        };

        for col in matrix_row.keys() {
            let value = col_values.get(col).copied().unwrap_or_default();
            if value > max_value && !rect.cols.contains(col) {
                max_value = value;
                best_col = Some(*col);
            }
        }
    }

    best_col
}

#[cfg(test)]
mod tests {
    use super::*;

    fn costs(values: &[MatrixCost]) -> CostTable {
        CostTable::new(values.to_vec())
    }

    fn matrix(entries: &[(MatrixIndex, MatrixIndex, MatrixValue)]) -> PingPongMatrix {
        let mut matrix = PingPongMatrix::new();
        for (row, col, value) in entries {
            matrix.insert(*row, *col, *value);
        }

        matrix
    }

    fn set(values: &[MatrixIndex]) -> BTreeSet<MatrixIndex> {
        values.iter().copied().collect()
    }

    #[test]
    fn chooses_seed_rows_and_columns_by_net_value() {
        let matrix = matrix(&[(0, 0, 2), (0, 1, 3), (1, 0, 8), (2, 2, 1)]);

        assert_eq!(choose_row_seed(&matrix, &costs(&[0, 5, 0])), Some(0));
        assert_eq!(choose_col_seed(&matrix, &costs(&[0, 4, 0])), Some(0));
    }

    #[test]
    fn onepass_returns_empty_when_no_two_row_or_column_rectangle_exists() {
        let matrix = matrix(&[(0, 0, 5), (0, 1, 5)]);
        let rect = ping_pong(&matrix, &CostTable::default(), &CostTable::default());

        assert_eq!(rect, Rectangle::new());
    }

    #[test]
    fn greedy_row_extracts_best_positive_shared_column_rectangle() {
        let matrix = matrix(&[(0, 0, 3), (0, 1, 3), (1, 0, 3), (1, 1, 3), (2, 1, 1)]);

        let rect = greedy_row(&matrix, &costs(&[1, 1, 1]), &costs(&[1, 1]), 0);

        assert_eq!(rect.rows(), &set(&[0, 1]));
        assert_eq!(rect.cols(), &set(&[0, 1]));
        assert_eq!(rect.value(), 8);
    }

    #[test]
    fn greedy_col_extracts_best_positive_shared_row_rectangle() {
        let matrix = matrix(&[(0, 0, 3), (0, 1, 3), (1, 0, 3), (1, 1, 3), (1, 2, 1)]);

        let rect = greedy_col(&matrix, &costs(&[1, 1, 1]), &costs(&[1, 1]), 0);

        assert_eq!(rect.rows(), &set(&[0, 1]));
        assert_eq!(rect.cols(), &set(&[0, 1]));
        assert_eq!(rect.value(), 8);
    }

    #[test]
    fn ping_pong_uses_column_pass_on_onepass_ties() {
        let matrix = matrix(&[(0, 0, 2), (0, 1, 2), (1, 0, 2), (1, 1, 2)]);
        let rect = ping_pong(&matrix, &CostTable::default(), &CostTable::default());

        assert_eq!(rect.rows(), &set(&[0, 1]));
        assert_eq!(rect.cols(), &set(&[0, 1]));
        assert_eq!(rect.value(), 8);
    }

    #[test]
    fn ping_pong_keeps_later_row_best_when_column_reply_does_not_beat_it() {
        let matrix = matrix(&[
            (0, 0, 4),
            (0, 1, 4),
            (1, 0, 4),
            (1, 1, 4),
            (2, 1, 5),
            (2, 2, 5),
        ]);

        let rect = ping_pong(&matrix, &costs(&[1, 1, 1]), &costs(&[1, 1, 1]));

        assert_eq!(rect.rows(), &set(&[0, 1]));
        assert_eq!(rect.cols(), &set(&[0, 1]));
        assert_eq!(rect.value(), 12);
    }

    #[test]
    fn update_best_continues_only_when_column_rectangle_beats_row_rectangle_and_best() {
        let mut best = Rectangle {
            rows: set(&[0]),
            cols: set(&[0]),
            value: 3,
        };
        let rect1 = Rectangle {
            rows: set(&[0, 1]),
            cols: set(&[0]),
            value: 4,
        };
        let rect2 = Rectangle {
            rows: set(&[0, 1]),
            cols: set(&[0, 1]),
            value: 5,
        };

        assert!(update_best(rect1, rect2.clone(), &mut best));
        assert_eq!(best, rect2);
    }
}
