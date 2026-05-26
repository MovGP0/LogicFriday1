//! Native Rust model for rectangle fanout collection in SIS extract.
//!
//! The C implementation walks every selected row and column in a rectangle,
//! reads the value cell at each sparse-matrix coordinate, and returns the
//! unique SIS fanout indexes. The cube variant additionally groups covered cube
//! numbers by fanout. This port keeps that behavior on owned Rust data and
//! reports malformed rectangle/matrix combinations explicitly.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SisIndex(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CubeNumber(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValueCell {
    pub value: usize,
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

    pub fn with_value(mut self, value: usize) -> Self {
        self.value = value;
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExtractionMatrix {
    cells: BTreeMap<(usize, usize), ValueCell>,
}

impl ExtractionMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, row: usize, column: usize, cell: ValueCell) -> Option<ValueCell> {
        self.cells.insert((row, column), cell)
    }

    pub fn cell(&self, row: usize, column: usize) -> Option<&ValueCell> {
        self.cells.get(&(row, column))
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Rectangle {
    rows: BTreeSet<usize>,
    columns: BTreeSet<usize>,
}

impl Rectangle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_rows_columns(
        rows: impl IntoIterator<Item = usize>,
        columns: impl IntoIterator<Item = usize>,
    ) -> Self {
        Self {
            rows: rows.into_iter().collect(),
            columns: columns.into_iter().collect(),
        }
    }

    pub fn insert_row(&mut self, row: usize) -> bool {
        self.rows.insert(row)
    }

    pub fn insert_column(&mut self, column: usize) -> bool {
        self.columns.insert(column)
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

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() || self.columns.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RectangleFanoutCubes {
    pub fanouts: Vec<SisIndex>,
    pub cubes: Vec<Vec<CubeNumber>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RectangleFanoutError {
    MissingMatrixCell { row: usize, column: usize },
}

impl fmt::Display for RectangleFanoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMatrixCell { row, column } => {
                write!(
                    f,
                    "rectangle references missing extraction matrix cell ({row}, {column})"
                )
            }
        }
    }
}

impl Error for RectangleFanoutError {}

pub fn find_rectangle_fanout(
    matrix: &ExtractionMatrix,
    rectangle: &Rectangle,
) -> Result<Vec<SisIndex>, RectangleFanoutError> {
    let mut fanouts = BTreeSet::new();

    for_each_rectangle_cell(matrix, rectangle, |cell| {
        fanouts.insert(cell.sis_index);
    })?;

    Ok(fanouts.into_iter().collect())
}

pub fn find_rectangle_fanout_cubes(
    matrix: &ExtractionMatrix,
    rectangle: &Rectangle,
) -> Result<RectangleFanoutCubes, RectangleFanoutError> {
    let mut cubes_by_fanout = BTreeMap::<SisIndex, BTreeSet<CubeNumber>>::new();

    for_each_rectangle_cell(matrix, rectangle, |cell| {
        cubes_by_fanout
            .entry(cell.sis_index)
            .or_default()
            .insert(cell.cube_number);
    })?;

    let mut fanouts = Vec::with_capacity(cubes_by_fanout.len());
    let mut cubes = Vec::with_capacity(cubes_by_fanout.len());
    for (fanout, fanout_cubes) in cubes_by_fanout {
        fanouts.push(fanout);
        cubes.push(fanout_cubes.into_iter().collect());
    }

    Ok(RectangleFanoutCubes { fanouts, cubes })
}

fn for_each_rectangle_cell(
    matrix: &ExtractionMatrix,
    rectangle: &Rectangle,
    mut visit: impl FnMut(&ValueCell),
) -> Result<(), RectangleFanoutError> {
    for row in rectangle.rows() {
        for column in rectangle.columns() {
            let cell = matrix
                .cell(row, column)
                .ok_or(RectangleFanoutError::MissingMatrixCell { row, column })?;
            visit(cell);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(fanout: usize, cube: usize) -> ValueCell {
        ValueCell::new(SisIndex(fanout), CubeNumber(cube))
    }

    fn sample_matrix() -> ExtractionMatrix {
        let mut matrix = ExtractionMatrix::new();
        matrix.insert(1, 2, cell(11, 0));
        matrix.insert(1, 3, cell(12, 4));
        matrix.insert(4, 2, cell(11, 1));
        matrix.insert(4, 3, cell(12, 4));
        matrix
    }

    #[test]
    fn fanout_collection_returns_unique_indexes_in_deterministic_order() {
        let matrix = sample_matrix();
        let rectangle = Rectangle::from_rows_columns([4, 1, 1], [3, 2]);

        let fanouts = find_rectangle_fanout(&matrix, &rectangle).unwrap();

        assert_eq!(fanouts, vec![SisIndex(11), SisIndex(12)]);
    }

    #[test]
    fn fanout_cube_collection_groups_unique_cubes_by_fanout() {
        let matrix = sample_matrix();
        let rectangle = Rectangle::from_rows_columns([1, 4], [2, 3]);

        let result = find_rectangle_fanout_cubes(&matrix, &rectangle).unwrap();

        assert_eq!(result.fanouts, vec![SisIndex(11), SisIndex(12)]);
        assert_eq!(
            result.cubes,
            vec![vec![CubeNumber(0), CubeNumber(1)], vec![CubeNumber(4)],]
        );
    }

    #[test]
    fn empty_rectangle_has_no_fanouts() {
        let matrix = sample_matrix();
        let rectangle = Rectangle::from_rows_columns([1, 4], []);

        assert_eq!(find_rectangle_fanout(&matrix, &rectangle), Ok(Vec::new()));
        assert_eq!(
            find_rectangle_fanout_cubes(&matrix, &rectangle),
            Ok(RectangleFanoutCubes {
                fanouts: Vec::new(),
                cubes: Vec::new(),
            })
        );
    }

    #[test]
    fn missing_matrix_cell_is_reported_instead_of_dereferencing_absent_data() {
        let matrix = sample_matrix();
        let rectangle = Rectangle::from_rows_columns([1, 4], [2, 5]);

        assert_eq!(
            find_rectangle_fanout(&matrix, &rectangle),
            Err(RectangleFanoutError::MissingMatrixCell { row: 1, column: 5 })
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("misc.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
