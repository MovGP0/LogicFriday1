//! Native Rust cube extraction support for SIS algebraic extraction.
//!
//! This module keeps the cube-literal matrix behavior owned and explicit:
//! rectangle selection is pure Rust, extracted cubes are reported through a
//! trait boundary, and matrix payloads carry the origin data that the legacy
//! sparse elements stored in user words.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SisIndex(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CubeNumber(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValueCell {
    pub value: i32,
    pub sis_index: SisIndex,
    pub cube_number: CubeNumber,
}

impl ValueCell {
    pub fn new(sis_index: SisIndex, cube_number: CubeNumber) -> Self {
        Self {
            value: 1,
            sis_index,
            cube_number,
        }
    }

    pub fn with_value(mut self, value: i32) -> Self {
        self.value = value;
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CubeLiteralMatrix {
    cells: BTreeMap<(usize, usize), ValueCell>,
}

impl CubeLiteralMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, row: usize, column: usize, cell: ValueCell) -> Option<ValueCell> {
        self.cells.insert((row, column), cell)
    }

    pub fn cell(&self, row: usize, column: usize) -> Option<&ValueCell> {
        self.cells.get(&(row, column))
    }

    pub fn remove(&mut self, row: usize, column: usize) -> Option<ValueCell> {
        self.cells.remove(&(row, column))
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn row_count(&self) -> usize {
        self.rows().count()
    }

    pub fn column_count(&self) -> usize {
        self.columns().count()
    }

    pub fn rows(&self) -> impl Iterator<Item = usize> + '_ {
        self.cells
            .keys()
            .map(|(row, _)| *row)
            .collect::<BTreeSet<_>>()
            .into_iter()
    }

    pub fn columns(&self) -> impl Iterator<Item = usize> + '_ {
        self.cells
            .keys()
            .map(|(_, column)| *column)
            .collect::<BTreeSet<_>>()
            .into_iter()
    }

    pub fn row_columns(&self, row: usize) -> Vec<usize> {
        self.cells
            .keys()
            .filter_map(|(candidate_row, column)| (*candidate_row == row).then_some(*column))
            .collect()
    }

    pub fn column_rows(&self, column: usize) -> Vec<usize> {
        self.cells
            .keys()
            .filter_map(|(row, candidate_column)| (*candidate_column == column).then_some(*row))
            .collect()
    }

    pub fn first_cell_in_row(&self, row: usize) -> Option<ValueCell> {
        self.cells
            .iter()
            .find_map(|((candidate_row, _), cell)| (*candidate_row == row).then_some(*cell))
    }

    pub fn last_row(&self) -> Option<usize> {
        self.cells.keys().map(|(row, _)| *row).max()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Rectangle {
    rows: BTreeSet<usize>,
    columns: BTreeSet<usize>,
    value: i32,
}

impl Rectangle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_rows_columns(
        rows: impl IntoIterator<Item = usize>,
        columns: impl IntoIterator<Item = usize>,
    ) -> Self {
        let mut rectangle = Self {
            rows: rows.into_iter().collect(),
            columns: columns.into_iter().collect(),
            value: 0,
        };
        rectangle.value = sop_value(rectangle.row_count(), rectangle.column_count());

        rectangle
    }

    pub fn rows(&self) -> impl Iterator<Item = usize> + '_ {
        self.rows.iter().copied()
    }

    pub fn columns(&self) -> impl Iterator<Item = usize> + '_ {
        self.columns.iter().copied()
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn with_value(mut self, value: i32) -> Self {
        self.value = value;
        self
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() || self.columns.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CubeExtractionStrategy {
    PingPong,
    BestSubcube,
    BestFactoredSubcube,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractedCube {
    pub columns: Vec<usize>,
    pub fanouts: Vec<SisIndex>,
}

pub trait CubeNetwork {
    fn divide_function_into_network(&mut self, cube: ExtractedCube) -> SisIndex;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecordingCubeNetwork {
    next_index: usize,
    extracted: Vec<ExtractedCube>,
}

impl RecordingCubeNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn extracted(&self) -> &[ExtractedCube] {
        &self.extracted
    }
}

impl CubeNetwork for RecordingCubeNetwork {
    fn divide_function_into_network(&mut self, cube: ExtractedCube) -> SisIndex {
        let index = SisIndex(self.next_index);
        self.next_index += 1;
        self.extracted.push(cube);

        index
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CubeExtractionError {
    MissingMatrixCell { row: usize, column: usize },
    MissingRowOrigin { row: usize },
}

impl fmt::Display for CubeExtractionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMatrixCell { row, column } => {
                write!(
                    f,
                    "rectangle references missing matrix cell ({row}, {column})"
                )
            }
            Self::MissingRowOrigin { row } => {
                write!(f, "row {row} has no origin cell")
            }
        }
    }
}

impl Error for CubeExtractionError {}

pub fn sparse_cube_extract(
    matrix: &mut CubeLiteralMatrix,
    value_threshold: i32,
    strategy: CubeExtractionStrategy,
    network: &mut impl CubeNetwork,
) -> Result<i32, CubeExtractionError> {
    sparse_cube_extract_with(matrix, value_threshold, network, |matrix| {
        choose_subcube(matrix, strategy)
    })
}

pub fn sparse_cube_extract_with(
    matrix: &mut CubeLiteralMatrix,
    value_threshold: i32,
    network: &mut impl CubeNetwork,
    mut choose: impl FnMut(&CubeLiteralMatrix) -> Rectangle,
) -> Result<i32, CubeExtractionError> {
    let mut total_value = 0;

    while !matrix.is_empty() {
        let rectangle = choose(matrix);
        if rectangle.row_count() < 2
            || rectangle.column_count() < 2
            || rectangle.value() <= value_threshold
        {
            break;
        }

        update_cube_literal_matrix(matrix, &rectangle, network)?;
        clear_rect(matrix, &rectangle)?;
        total_value += rectangle.value();
    }

    Ok(total_value)
}

pub fn choose_subcube(matrix: &CubeLiteralMatrix, strategy: CubeExtractionStrategy) -> Rectangle {
    match strategy {
        CubeExtractionStrategy::PingPong => ping_pong(matrix),
        CubeExtractionStrategy::BestSubcube => best_subcube(matrix),
        CubeExtractionStrategy::BestFactoredSubcube => best_factored_subcube(matrix),
    }
}

pub fn sop_value(rows: usize, columns: usize) -> i32 {
    (rows as i32 - 1) * (columns as i32 - 1) - 1
}

pub fn best_subcube(matrix: &CubeLiteralMatrix) -> Rectangle {
    choose_best_rectangle(generate_all_rectangles(matrix))
}

pub fn best_factored_subcube(matrix: &CubeLiteralMatrix) -> Rectangle {
    choose_best_rectangle(
        generate_all_rectangles(matrix)
            .into_iter()
            .map(|rectangle| {
                let function_count = rectangle
                    .rows()
                    .filter_map(|row| matrix.first_cell_in_row(row).map(|cell| cell.sis_index))
                    .collect::<BTreeSet<_>>()
                    .len();
                let value = (rectangle.column_count() as i32 - 1) * (function_count as i32 - 1) - 1;

                rectangle.with_value(value)
            }),
    )
}

pub fn ping_pong(matrix: &CubeLiteralMatrix) -> Rectangle {
    let mut best = Rectangle::new();

    if let Some(seed_row) = choose_row_seed(matrix) {
        best = better_rect(best, greedy_row(matrix, seed_row));
    }

    if let Some(seed_column) = choose_column_seed(matrix) {
        best = better_rect(best, greedy_column(matrix, seed_column));
    }

    best
}

fn update_cube_literal_matrix(
    matrix: &mut CubeLiteralMatrix,
    rectangle: &Rectangle,
    network: &mut impl CubeNetwork,
) -> Result<(), CubeExtractionError> {
    let extracted = ExtractedCube {
        columns: rectangle.columns().collect(),
        fanouts: find_rectangle_fanout(matrix, rectangle)?,
    };
    let new_index = network.divide_function_into_network(extracted);
    let new_column = new_index.0 * 2;

    let rows = rectangle.rows().collect::<Vec<_>>();
    for row in rows {
        let old_cell = matrix
            .first_cell_in_row(row)
            .ok_or(CubeExtractionError::MissingRowOrigin { row })?;

        matrix.insert(
            row,
            new_column,
            ValueCell {
                value: 1,
                sis_index: old_cell.sis_index,
                cube_number: old_cell.cube_number,
            },
        );
    }

    let new_row = matrix.last_row().unwrap_or(0) + 1;
    for column in rectangle.columns() {
        matrix.insert(
            new_row,
            column,
            ValueCell {
                value: 1,
                sis_index: new_index,
                cube_number: CubeNumber(0),
            },
        );
    }

    Ok(())
}

fn clear_rect(
    matrix: &mut CubeLiteralMatrix,
    rectangle: &Rectangle,
) -> Result<(), CubeExtractionError> {
    for row in rectangle.rows() {
        for column in rectangle.columns() {
            matrix
                .remove(row, column)
                .ok_or(CubeExtractionError::MissingMatrixCell { row, column })?;
        }
    }

    Ok(())
}

fn find_rectangle_fanout(
    matrix: &CubeLiteralMatrix,
    rectangle: &Rectangle,
) -> Result<Vec<SisIndex>, CubeExtractionError> {
    let mut fanouts = BTreeSet::new();

    for row in rectangle.rows() {
        for column in rectangle.columns() {
            let cell = matrix
                .cell(row, column)
                .ok_or(CubeExtractionError::MissingMatrixCell { row, column })?;
            fanouts.insert(cell.sis_index);
        }
    }

    Ok(fanouts.into_iter().collect())
}

fn generate_all_rectangles(matrix: &CubeLiteralMatrix) -> Vec<Rectangle> {
    let mut rectangles = Vec::new();
    generate_all_rectangles_from(matrix, &Rectangle::new(), 0, &mut rectangles);

    if !has_full_column(matrix) {
        rectangles.push(Rectangle::new());
    }

    rectangles
}

fn generate_all_rectangles_from(
    matrix: &CubeLiteralMatrix,
    rectangle: &Rectangle,
    min_column: usize,
    rectangles: &mut Vec<Rectangle>,
) {
    for column in matrix.columns().collect::<Vec<_>>() {
        let column_rows = matrix.column_rows(column);
        if column_rows.len() < 2 || column < min_column {
            continue;
        }

        let mut submatrix = rows_induced_by_column(matrix, &column_rows);
        let mut subrectangle = Rectangle {
            rows: column_rows.iter().copied().collect(),
            columns: rectangle.columns.clone(),
            value: 0,
        };
        let mut already_generated = false;

        for full_column in submatrix.columns().collect::<Vec<_>>() {
            if submatrix.column_rows(full_column).len() == column_rows.len() {
                if full_column < column {
                    already_generated = true;
                    break;
                }

                subrectangle.columns.insert(full_column);
                delete_column(&mut submatrix, full_column);
            }
        }

        subrectangle.value = sop_value(subrectangle.row_count(), subrectangle.column_count());
        if !already_generated {
            rectangles.push(subrectangle.clone());
            generate_all_rectangles_from(&submatrix, &subrectangle, column, rectangles);
        }
    }
}

fn rows_induced_by_column(matrix: &CubeLiteralMatrix, rows: &[usize]) -> CubeLiteralMatrix {
    let row_set = rows.iter().copied().collect::<BTreeSet<_>>();
    let mut submatrix = CubeLiteralMatrix::new();

    for ((row, column), cell) in &matrix.cells {
        if row_set.contains(row) {
            submatrix.insert(*row, *column, *cell);
        }
    }

    submatrix
}

fn has_full_column(matrix: &CubeLiteralMatrix) -> bool {
    let row_count = matrix.row_count();
    row_count > 0
        && matrix
            .columns()
            .any(|column| matrix.column_rows(column).len() == row_count)
}

fn delete_column(matrix: &mut CubeLiteralMatrix, column: usize) {
    let rows = matrix.column_rows(column);
    for row in rows {
        matrix.remove(row, column);
    }
}

fn choose_row_seed(matrix: &CubeLiteralMatrix) -> Option<usize> {
    matrix.rows().max_by_key(|row| row_value(matrix, *row))
}

fn choose_column_seed(matrix: &CubeLiteralMatrix) -> Option<usize> {
    matrix
        .columns()
        .max_by_key(|column| column_value(matrix, *column))
}

fn row_value(matrix: &CubeLiteralMatrix, row: usize) -> i32 {
    matrix
        .row_columns(row)
        .iter()
        .filter_map(|column| matrix.cell(row, *column))
        .map(|cell| cell.value)
        .sum::<i32>()
        - 1
}

fn column_value(matrix: &CubeLiteralMatrix, column: usize) -> i32 {
    matrix
        .column_rows(column)
        .iter()
        .filter_map(|row| matrix.cell(*row, column))
        .map(|cell| cell.value)
        .sum::<i32>()
        - 1
}

fn greedy_row(matrix: &CubeLiteralMatrix, seed_row: usize) -> Rectangle {
    let seed_columns = matrix
        .row_columns(seed_row)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let candidates = matrix
        .rows()
        .filter(|row| *row != seed_row && intersects(&seed_columns, &matrix.row_columns(*row)))
        .collect::<Vec<_>>();
    let mut best = Rectangle::new();

    enumerate_row_sets(
        matrix,
        &[seed_row],
        &candidates,
        0,
        &seed_columns,
        &mut best,
    );

    best
}

fn enumerate_row_sets(
    matrix: &CubeLiteralMatrix,
    rows: &[usize],
    candidates: &[usize],
    index: usize,
    common_columns: &BTreeSet<usize>,
    best: &mut Rectangle,
) {
    if rows.len() >= 2 && common_columns.len() >= 2 {
        *best = better_rect(
            best.clone(),
            Rectangle::from_rows_columns(rows.iter().copied(), common_columns.iter().copied()),
        );
    }

    for candidate_index in index..candidates.len() {
        let candidate = candidates[candidate_index];
        let next_columns = common_columns
            .intersection(&matrix.row_columns(candidate).into_iter().collect())
            .copied()
            .collect::<BTreeSet<_>>();

        if next_columns.len() < 2 {
            continue;
        }

        let mut next_rows = rows.to_vec();
        next_rows.push(candidate);
        enumerate_row_sets(
            matrix,
            &next_rows,
            candidates,
            candidate_index + 1,
            &next_columns,
            best,
        );
    }
}

fn greedy_column(matrix: &CubeLiteralMatrix, seed_column: usize) -> Rectangle {
    let seed_rows = matrix
        .column_rows(seed_column)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let candidates = matrix
        .columns()
        .filter(|column| {
            *column != seed_column && intersects(&seed_rows, &matrix.column_rows(*column))
        })
        .collect::<Vec<_>>();
    let mut best = Rectangle::new();

    enumerate_column_sets(
        matrix,
        &[seed_column],
        &candidates,
        0,
        &seed_rows,
        &mut best,
    );

    best
}

fn enumerate_column_sets(
    matrix: &CubeLiteralMatrix,
    columns: &[usize],
    candidates: &[usize],
    index: usize,
    common_rows: &BTreeSet<usize>,
    best: &mut Rectangle,
) {
    if common_rows.len() >= 2 && columns.len() >= 2 {
        *best = better_rect(
            best.clone(),
            Rectangle::from_rows_columns(common_rows.iter().copied(), columns.iter().copied()),
        );
    }

    for candidate_index in index..candidates.len() {
        let candidate = candidates[candidate_index];
        let next_rows = common_rows
            .intersection(&matrix.column_rows(candidate).into_iter().collect())
            .copied()
            .collect::<BTreeSet<_>>();

        if next_rows.len() < 2 {
            continue;
        }

        let mut next_columns = columns.to_vec();
        next_columns.push(candidate);
        enumerate_column_sets(
            matrix,
            &next_columns,
            candidates,
            candidate_index + 1,
            &next_rows,
            best,
        );
    }
}

fn better_rect(left: Rectangle, right: Rectangle) -> Rectangle {
    if left.value() > right.value() {
        left
    } else {
        right
    }
}

fn choose_best_rectangle(rectangles: impl IntoIterator<Item = Rectangle>) -> Rectangle {
    let mut best = Rectangle::new();

    for rectangle in rectangles {
        if best.row_count() == 0 || rectangle.value() > best.value() {
            best = rectangle;
        }
    }

    best
}

fn intersects(left: &BTreeSet<usize>, right: &[usize]) -> bool {
    right.iter().any(|value| left.contains(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(fanout: usize, cube: usize) -> ValueCell {
        ValueCell::new(SisIndex(fanout), CubeNumber(cube))
    }

    fn sample_matrix() -> CubeLiteralMatrix {
        let mut matrix = CubeLiteralMatrix::new();
        matrix.insert(0, 0, cell(10, 0));
        matrix.insert(0, 1, cell(10, 0));
        matrix.insert(0, 2, cell(10, 0));
        matrix.insert(1, 0, cell(11, 1));
        matrix.insert(1, 1, cell(11, 1));
        matrix.insert(2, 0, cell(12, 2));
        matrix.insert(2, 2, cell(12, 2));
        matrix
    }

    #[test]
    fn sop_value_matches_legacy_formula() {
        assert_eq!(sop_value(2, 2), 0);
        assert_eq!(sop_value(3, 3), 3);
        assert_eq!(sop_value(4, 2), 2);
    }

    #[test]
    fn best_subcube_finds_highest_value_rectangle() {
        let rectangle = best_subcube(&sample_matrix());

        assert_eq!(rectangle.rows().collect::<Vec<_>>(), vec![0, 1]);
        assert_eq!(rectangle.columns().collect::<Vec<_>>(), vec![0, 1]);
        assert_eq!(rectangle.value(), sop_value(2, 2));
    }

    #[test]
    fn best_factored_subcube_uses_unique_function_count() {
        let mut matrix = CubeLiteralMatrix::new();
        matrix.insert(0, 0, cell(10, 0));
        matrix.insert(0, 1, cell(10, 0));
        matrix.insert(1, 0, cell(10, 1));
        matrix.insert(1, 1, cell(10, 1));
        matrix.insert(2, 0, cell(11, 0));
        matrix.insert(2, 1, cell(11, 0));

        let rectangle = best_factored_subcube(&matrix);

        assert_eq!(rectangle.rows().collect::<Vec<_>>(), vec![0, 1, 2]);
        assert_eq!(rectangle.columns().collect::<Vec<_>>(), vec![0, 1]);
        assert_eq!(rectangle.value(), 0);
    }

    #[test]
    fn extraction_inserts_new_factor_column_and_cube_row_then_clears_rectangle() {
        let mut matrix = CubeLiteralMatrix::new();
        matrix.insert(0, 3, cell(20, 0));
        matrix.insert(0, 5, cell(20, 0));
        matrix.insert(0, 7, cell(20, 0));
        matrix.insert(1, 3, cell(21, 4));
        matrix.insert(1, 5, cell(21, 4));
        matrix.insert(1, 7, cell(21, 4));
        let mut network = RecordingCubeNetwork::new();

        let total = sparse_cube_extract(
            &mut matrix,
            0,
            CubeExtractionStrategy::BestSubcube,
            &mut network,
        )
        .unwrap();

        assert_eq!(total, 1);
        assert_eq!(
            network.extracted(),
            &[ExtractedCube {
                columns: vec![3, 5, 7],
                fanouts: vec![SisIndex(20), SisIndex(21)],
            }]
        );
        assert!(matrix.cell(0, 3).is_none());
        assert!(matrix.cell(1, 5).is_none());
        assert_eq!(matrix.cell(0, 0), Some(&cell(20, 0)));
        assert_eq!(matrix.cell(1, 0), Some(&cell(21, 4)));
        assert_eq!(matrix.cell(2, 3), Some(&cell(0, 0)));
        assert_eq!(matrix.cell(2, 5), Some(&cell(0, 0)));
        assert_eq!(matrix.cell(2, 7), Some(&cell(0, 0)));
    }

    #[test]
    fn threshold_stops_extraction_without_mutating_matrix() {
        let mut matrix = sample_matrix();
        let original = matrix.clone();
        let mut network = RecordingCubeNetwork::new();

        let total = sparse_cube_extract(
            &mut matrix,
            10,
            CubeExtractionStrategy::BestSubcube,
            &mut network,
        )
        .unwrap();

        assert_eq!(total, 0);
        assert_eq!(matrix, original);
        assert!(network.extracted().is_empty());
    }

    #[test]
    fn custom_chooser_reports_missing_rectangle_cells() {
        let mut matrix = CubeLiteralMatrix::new();
        matrix.insert(0, 0, cell(1, 0));
        matrix.insert(1, 0, cell(2, 0));
        let mut network = RecordingCubeNetwork::new();

        let result = sparse_cube_extract_with(&mut matrix, -10, &mut network, |_| {
            Rectangle::from_rows_columns([0, 1], [0, 9]).with_value(1)
        });

        assert_eq!(
            result,
            Err(CubeExtractionError::MissingMatrixCell { row: 0, column: 9 })
        );
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let source = include_str!("cube.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
    }
}
