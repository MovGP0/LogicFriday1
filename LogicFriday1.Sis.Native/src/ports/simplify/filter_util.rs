//! Native Rust model for `LogicSynthesis/sis/simplify/filter_util.c`.
//!
//! The C file filters don't-care rows in an `sm_matrix` using sparse row,
//! column, flag, and `user_word` metadata. This module ports those filtering
//! algorithms onto an owned Rust matrix model. Binding these functions directly
//! to the canonical native sparse matrix remains blocked until the sparse row,
//! column, and matrix ports expose the same metadata surface.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const SIZE_BOUND: usize = 3;
pub const DIST_BOUND: i32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RowKind {
    OnSet,
    DontCare,
}

pub const F_SET: RowKind = RowKind::OnSet;
pub const DC_SET: RowKind = RowKind::DontCare;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElementValue {
    Zero,
    One,
    DontCare,
}

impl ElementValue {
    fn is_specified(self) -> bool {
        self != Self::DontCare
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FilterRow {
    kind: RowKind,
    elements: BTreeMap<usize, ElementValue>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FilterMatrix {
    rows: BTreeMap<usize, FilterRow>,
    cols: BTreeSet<usize>,
    rows_size: usize,
    cols_size: usize,
}

impl FilterMatrix {
    pub fn new() -> Self {
        Self::default()
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

    pub fn row_kind(&self, row: usize) -> Option<RowKind> {
        self.rows.get(&row).map(|row| row.kind)
    }

    pub fn row_length(&self, row: usize) -> Option<usize> {
        self.rows.get(&row).map(|row| row.elements.len())
    }

    pub fn element(&self, row: usize, col: usize) -> Option<ElementValue> {
        self.rows
            .get(&row)
            .and_then(|row| row.elements.get(&col).copied())
    }

    pub fn rows(&self) -> impl Iterator<Item = usize> + '_ {
        self.rows.keys().copied()
    }

    pub fn cols(&self) -> impl Iterator<Item = usize> + '_ {
        self.cols.iter().copied()
    }

    pub fn add_row(&mut self, row: usize, kind: RowKind) {
        self.rows.entry(row).or_insert_with(|| FilterRow {
            kind,
            elements: BTreeMap::new(),
        });
        self.rows_size = self.rows_size.max(row + 1);
    }

    pub fn insert(&mut self, row: usize, col: usize, value: ElementValue, kind: RowKind) {
        self.add_row(row, kind);
        self.rows
            .get_mut(&row)
            .expect("row was inserted above")
            .elements
            .insert(col, value);
        self.cols.insert(col);
        self.rows_size = self.rows_size.max(row + 1);
        self.cols_size = self.cols_size.max(col + 1);
    }

    pub fn delete_row(&mut self, row: usize) -> bool {
        let Some(removed) = self.rows.remove(&row) else {
            return false;
        };

        for col in removed.elements.keys() {
            if !self.rows.values().any(|row| row.elements.contains_key(col)) {
                self.cols.remove(col);
            }
        }
        true
    }

    pub fn retained_rows(&self) -> Vec<usize> {
        self.rows.keys().copied().collect()
    }

    fn row_elements(&self, row: usize) -> impl Iterator<Item = (usize, ElementValue)> + '_ {
        self.rows
            .get(&row)
            .into_iter()
            .flat_map(|row| row.elements.iter().map(|(col, value)| (*col, *value)))
    }

    fn col_rows(&self, col: usize) -> impl Iterator<Item = usize> + '_ {
        self.rows.iter().filter_map(move |(row_index, row)| {
            row.elements.contains_key(&col).then_some(*row_index)
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FilterUtilError {
    MissingSisPorts { operation: &'static str },
    UnknownRow(usize),
}

impl fmt::Display for FilterUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisPorts { operation } => write!(
                f,
                "{operation} requires native Rust SIS ports that are not available yet"
            ),
            Self::UnknownRow(row) => write!(f, "unknown sparse matrix row {row}"),
        }
    }
}

impl Error for FilterUtilError {}

pub fn sis_sparse_matrix_filter_unavailable(
    operation: &'static str,
) -> Result<(), FilterUtilError> {
    Err(FilterUtilError::MissingSisPorts { operation })
}

pub fn exact_block_filter_excluding_col(matrix: &mut FilterMatrix, excluded_col: Option<usize>) {
    if matrix.row_count() == 0 {
        return;
    }

    let mut row_flags = matrix
        .rows
        .iter()
        .map(|(row, data)| (*row, u8::from(data.kind == RowKind::OnSet)))
        .collect::<BTreeMap<_, _>>();
    let mut col_flags = matrix.cols.iter().map(|col| (*col, 0_u8)).collect();

    block_decompose_exact(matrix, excluded_col, &mut row_flags, &mut col_flags);

    for row in 0..matrix.rows_size() {
        if matrix.rows.contains_key(&row) && row_flags.get(&row).copied().unwrap_or(0) == 0 {
            matrix.delete_row(row);
        }
    }
}

fn block_decompose_exact(
    matrix: &FilterMatrix,
    excluded_col: Option<usize>,
    row_flags: &mut BTreeMap<usize, u8>,
    col_flags: &mut BTreeMap<usize, u8>,
) {
    loop {
        let mut new_col_found = false;
        for row in matrix.rows() {
            if row_flags.get(&row).copied() == Some(1) {
                row_flags.insert(row, 2);
                for (col, _) in matrix.row_elements(row) {
                    if Some(col) != excluded_col && col_flags.get(&col).copied().unwrap_or(0) == 0 {
                        new_col_found = true;
                        col_flags.insert(col, 1);
                    }
                }
            }
        }

        if !new_col_found {
            return;
        }

        let mut new_row_found = false;
        for col in matrix.cols() {
            if Some(col) != excluded_col && col_flags.get(&col).copied() == Some(1) {
                col_flags.insert(col, 2);
                for row in matrix.col_rows(col) {
                    if row_flags.get(&row).copied().unwrap_or(0) == 0 {
                        new_row_found = true;
                        row_flags.insert(row, 1);
                    }
                }
            }
        }

        if !new_row_found {
            return;
        }
    }
}

pub fn disjoint_support_filter(matrix: &mut FilterMatrix) {
    if matrix.row_count() == 0 {
        return;
    }

    let on_support = on_set_support(matrix);
    let mut keep_rows = matrix
        .rows
        .iter()
        .filter_map(|(row, data)| (data.kind == RowKind::OnSet).then_some(*row))
        .collect::<BTreeSet<_>>();

    for col in on_support {
        keep_rows.extend(matrix.col_rows(col));
    }

    for row in 0..matrix.rows_size() {
        if matrix.rows.contains_key(&row) && !keep_rows.contains(&row) {
            matrix.delete_row(row);
        }
    }
}

pub fn size_filter(matrix: &mut FilterMatrix) {
    if matrix.row_count() == 0 {
        return;
    }

    let on_support = on_set_support(matrix);
    let mut delete_rows = BTreeSet::new();
    for col in matrix.cols() {
        if !on_support.contains(&col) {
            delete_rows.extend(matrix.col_rows(col));
        }
    }

    delete_rows.retain(|row| matrix.row_length(*row).unwrap_or(0) > SIZE_BOUND);

    for row in 0..matrix.rows_size() {
        if delete_rows.contains(&row) {
            matrix.delete_row(row);
        }
    }
}

pub fn first_distance_filter(matrix: &mut FilterMatrix, distance: i32) {
    if matrix.row_count() == 0 {
        return;
    }

    let on_support = on_set_support(matrix);
    let mut keep_rows = matrix
        .rows
        .iter()
        .filter_map(|(row, data)| (data.kind == RowKind::OnSet).then_some(*row))
        .collect::<BTreeSet<_>>();

    for row in matrix.rows() {
        if matrix.row_kind(row) != Some(RowKind::DontCare) {
            continue;
        }
        let mut current_distance = 0;
        for (col, value) in matrix.row_elements(row) {
            if !on_support.contains(&col) && value.is_specified() {
                current_distance += 1;
            }
            if current_distance > distance {
                break;
            }
        }
        if current_distance <= distance {
            keep_rows.insert(row);
        }
    }

    for row in 0..matrix.rows_size() {
        if matrix.rows.contains_key(&row) && !keep_rows.contains(&row) {
            matrix.delete_row(row);
        }
    }
}

pub fn support_distance_filter(matrix: &mut FilterMatrix, distance: i32) {
    if matrix.row_count() == 0 {
        return;
    }

    let on_support = on_set_support(matrix);
    let mut keep_rows = matrix
        .rows
        .iter()
        .filter_map(|(row, data)| (data.kind == RowKind::OnSet).then_some(*row))
        .collect::<BTreeSet<_>>();
    let on_rows = keep_rows.iter().copied().collect::<Vec<_>>();
    let dc_rows = matrix
        .rows
        .iter()
        .filter_map(|(row, data)| (data.kind == RowKind::DontCare).then_some(*row))
        .collect::<Vec<_>>();

    for on_row in on_rows {
        for dc_row in &dc_rows {
            if keep_rows.contains(dc_row) {
                continue;
            }
            if distance_check_with_support(matrix, on_row, *dc_row, distance, &on_support)
                .unwrap_or(false)
            {
                keep_rows.insert(*dc_row);
            }
        }
    }

    for row in 0..matrix.rows_size() {
        if matrix.rows.contains_key(&row) && !keep_rows.contains(&row) {
            matrix.delete_row(row);
        }
    }
}

pub fn col_count_init(size: usize) -> Vec<bool> {
    vec![false; size]
}

pub fn get_long_col(matrix: &FilterMatrix, chosen: &mut [bool]) -> Option<usize> {
    let mut best = None;
    for col in matrix.cols() {
        if chosen.get(col).copied().unwrap_or(false) {
            continue;
        }
        let length = matrix.col_rows(col).count();
        if best
            .map(|(_, best_length)| length > best_length)
            .unwrap_or(true)
        {
            best = Some((col, length));
        }
    }

    let (col, _) = best?;
    if let Some(slot) = chosen.get_mut(col) {
        *slot = true;
    }
    Some(col)
}

pub fn distance_check(
    matrix: &FilterMatrix,
    on_row: usize,
    dc_row: usize,
    distance: i32,
) -> Result<bool, FilterUtilError> {
    let on_support = on_set_support(matrix);
    distance_check_with_support(matrix, on_row, dc_row, distance, &on_support)
}

fn distance_check_with_support(
    matrix: &FilterMatrix,
    on_row: usize,
    dc_row: usize,
    distance: i32,
    on_support: &BTreeSet<usize>,
) -> Result<bool, FilterUtilError> {
    let Some(on_data) = matrix.rows.get(&on_row) else {
        return Err(FilterUtilError::UnknownRow(on_row));
    };
    let Some(dc_data) = matrix.rows.get(&dc_row) else {
        return Err(FilterUtilError::UnknownRow(dc_row));
    };

    let mut on_iter = on_data.elements.iter().peekable();
    let mut dc_iter = dc_data.elements.iter().peekable();
    let mut current_distance = 0;

    while let (Some((on_col, on_value)), Some((dc_col, dc_value))) =
        (on_iter.peek(), dc_iter.peek())
    {
        if on_col < dc_col {
            on_iter.next();
        } else if on_col > dc_col {
            if !on_support.contains(dc_col) && dc_value.is_specified() {
                current_distance += 1;
            }
            dc_iter.next();
        } else {
            if on_support.contains(on_col) {
                if on_value.is_specified() && dc_value.is_specified() && on_value != dc_value {
                    current_distance += 1;
                }
            } else if dc_value.is_specified() && on_value != dc_value {
                current_distance += 1;
            }
            on_iter.next();
            dc_iter.next();
        }

        if current_distance > distance {
            return Ok(false);
        }
    }

    for _ in dc_iter {
        current_distance += 1;
        if current_distance > distance {
            return Ok(false);
        }
    }

    Ok(true)
}

fn on_set_support(matrix: &FilterMatrix) -> BTreeSet<usize> {
    matrix
        .rows
        .values()
        .filter(|row| row.kind == RowKind::OnSet)
        .flat_map(|row| row.elements.keys().copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(rows: &[(usize, RowKind, &[(usize, ElementValue)])]) -> FilterMatrix {
        let mut matrix = FilterMatrix::new();
        for (row, kind, elements) in rows {
            matrix.add_row(*row, *kind);
            for (col, value) in *elements {
                matrix.insert(*row, *col, *value, *kind);
            }
        }
        matrix
    }

    #[test]
    fn exact_filter_keeps_component_connected_to_on_set_while_ignoring_column() {
        let mut matrix = matrix(&[
            (0, F_SET, &[(0, ElementValue::One)]),
            (
                1,
                DC_SET,
                &[(0, ElementValue::Zero), (1, ElementValue::One)],
            ),
            (2, DC_SET, &[(2, ElementValue::One)]),
            (3, DC_SET, &[(1, ElementValue::One)]),
        ]);

        exact_block_filter_excluding_col(&mut matrix, Some(1));

        assert_eq!(matrix.retained_rows(), vec![0, 1]);
    }

    #[test]
    fn disjoint_support_filter_deletes_dc_rows_without_on_set_columns() {
        let mut matrix = matrix(&[
            (0, F_SET, &[(0, ElementValue::One)]),
            (1, DC_SET, &[(0, ElementValue::Zero)]),
            (2, DC_SET, &[(2, ElementValue::One)]),
        ]);

        disjoint_support_filter(&mut matrix);

        assert_eq!(matrix.retained_rows(), vec![0, 1]);
    }

    #[test]
    fn size_filter_deletes_large_dc_rows_with_support_outside_on_set() {
        let mut matrix = matrix(&[
            (0, F_SET, &[(0, ElementValue::One)]),
            (
                1,
                DC_SET,
                &[
                    (0, ElementValue::One),
                    (1, ElementValue::One),
                    (2, ElementValue::One),
                    (3, ElementValue::One),
                ],
            ),
            (2, DC_SET, &[(1, ElementValue::One), (2, ElementValue::One)]),
        ]);

        size_filter(&mut matrix);

        assert_eq!(matrix.retained_rows(), vec![0, 2]);
    }

    #[test]
    fn first_distance_filter_uses_only_support_outside_on_set() {
        let mut matrix = matrix(&[
            (0, F_SET, &[(0, ElementValue::One)]),
            (
                1,
                DC_SET,
                &[(0, ElementValue::Zero), (1, ElementValue::One)],
            ),
            (
                2,
                DC_SET,
                &[
                    (1, ElementValue::One),
                    (2, ElementValue::One),
                    (3, ElementValue::DontCare),
                ],
            ),
        ]);

        first_distance_filter(&mut matrix, 1);

        assert_eq!(matrix.retained_rows(), vec![0, 1]);
    }

    #[test]
    fn support_distance_filter_checks_each_on_set_cube() {
        let mut matrix = matrix(&[
            (0, F_SET, &[(0, ElementValue::One), (1, ElementValue::Zero)]),
            (
                1,
                F_SET,
                &[(0, ElementValue::Zero), (1, ElementValue::Zero)],
            ),
            (2, DC_SET, &[(0, ElementValue::One), (1, ElementValue::One)]),
            (
                3,
                DC_SET,
                &[(0, ElementValue::Zero), (1, ElementValue::One)],
            ),
        ]);

        support_distance_filter(&mut matrix, 1);

        assert_eq!(matrix.retained_rows(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn get_long_col_skips_and_marks_chosen_columns() {
        let matrix = matrix(&[
            (0, F_SET, &[(0, ElementValue::One), (2, ElementValue::Zero)]),
            (1, DC_SET, &[(2, ElementValue::One)]),
            (2, DC_SET, &[(1, ElementValue::One), (2, ElementValue::One)]),
        ]);
        let mut chosen = col_count_init(matrix.cols_size());

        assert_eq!(get_long_col(&matrix, &mut chosen), Some(2));
        assert!(chosen[2]);
        assert_eq!(get_long_col(&matrix, &mut chosen), Some(0));
    }

    #[test]
    fn distance_check_matches_c_element_walk_rules() {
        let matrix = matrix(&[
            (
                0,
                F_SET,
                &[(0, ElementValue::One), (2, ElementValue::DontCare)],
            ),
            (
                1,
                DC_SET,
                &[
                    (0, ElementValue::Zero),
                    (1, ElementValue::One),
                    (2, ElementValue::One),
                    (3, ElementValue::DontCare),
                ],
            ),
        ]);

        assert_eq!(distance_check(&matrix, 0, 1, 3), Ok(true));
        assert_eq!(distance_check(&matrix, 0, 1, 2), Ok(false));
    }

    #[test]
    fn sis_sparse_matrix_entry_reports_missing_sis_ports() {
        let err = sis_sparse_matrix_filter_unavailable("fdc_sm_bp_1")
            .expect_err("canonical SIS sm_matrix integration should be blocked");

        match err {
            FilterUtilError::MissingSisPorts { operation } => {
                assert_eq!(operation, "fdc_sm_bp_1");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
