//! Native greedy row rectangle extraction.
//!
//! The algorithm grows a seed row into a valued rectangle by repeatedly adding
//! the row with the best positive intersection against the current columns,
//! then shrinking the column set to the common columns of all selected rows.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RowId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ColumnId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WeightedElement {
    pub row: RowId,
    pub column: ColumnId,
    pub value: i32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WeightedMatrix {
    rows: BTreeMap<RowId, BTreeMap<ColumnId, i32>>,
    columns: BTreeMap<ColumnId, BTreeMap<RowId, i32>>,
}

impl WeightedMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, row: RowId, column: ColumnId, value: i32) -> Option<i32> {
        let old_value = self.rows.entry(row).or_default().insert(column, value);
        self.columns.entry(column).or_default().insert(row, value);
        old_value
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn row(&self, row: RowId) -> Option<WeightedRow<'_>> {
        self.rows.get(&row).map(|elements| WeightedRow {
            index: row,
            elements,
        })
    }

    pub fn column(&self, column: ColumnId) -> Option<WeightedColumn<'_>> {
        self.columns.get(&column).map(|elements| WeightedColumn {
            index: column,
            elements,
        })
    }

    pub fn find(&self, row: RowId, column: ColumnId) -> Option<WeightedElement> {
        self.rows
            .get(&row)
            .and_then(|columns| columns.get(&column))
            .copied()
            .map(|value| WeightedElement { row, column, value })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WeightedRow<'a> {
    index: RowId,
    elements: &'a BTreeMap<ColumnId, i32>,
}

impl WeightedRow<'_> {
    pub fn index(&self) -> RowId {
        self.index
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn elements(&self) -> impl Iterator<Item = WeightedElement> + '_ {
        self.elements.iter().map(|(column, value)| WeightedElement {
            row: self.index,
            column: *column,
            value: *value,
        })
    }

    pub fn contains(&self, column: ColumnId) -> bool {
        self.elements.contains_key(&column)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WeightedColumn<'a> {
    index: ColumnId,
    elements: &'a BTreeMap<RowId, i32>,
}

impl WeightedColumn<'_> {
    pub fn index(&self) -> ColumnId {
        self.index
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn elements(&self) -> impl Iterator<Item = WeightedElement> + '_ {
        self.elements.iter().map(|(row, value)| WeightedElement {
            row: *row,
            column: self.index,
            value: *value,
        })
    }

    pub fn contains(&self, row: RowId) -> bool {
        self.elements.contains_key(&row)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GreedyRectangle {
    rows: BTreeSet<RowId>,
    columns: BTreeSet<ColumnId>,
    value: i32,
}

impl GreedyRectangle {
    pub fn rows(&self) -> &BTreeSet<RowId> {
        &self.rows
    }

    pub fn columns(&self) -> &BTreeSet<ColumnId> {
        &self.columns
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn row_len(&self) -> usize {
        self.rows.len()
    }

    pub fn column_len(&self) -> usize {
        self.columns.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GreedyRowError {
    SeedRowNotFound(RowId),
    MissingRowCost(RowId),
    MissingColumnCost(ColumnId),
    MatrixMissingElement { row: RowId, column: ColumnId },
}

impl fmt::Display for GreedyRowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SeedRowNotFound(row) => write!(f, "seed row {} is not present", row.0),
            Self::MissingRowCost(row) => write!(f, "missing row cost for row {}", row.0),
            Self::MissingColumnCost(column) => {
                write!(f, "missing column cost for column {}", column.0)
            }
            Self::MatrixMissingElement { row, column } => write!(
                f,
                "matrix is missing element at row {}, column {}",
                row.0, column.0
            ),
        }
    }
}

impl Error for GreedyRowError {}

pub type GreedyRowResult<T> = Result<T, GreedyRowError>;

pub fn greedy_row(
    matrix: &WeightedMatrix,
    row_costs: &[i32],
    column_costs: &[i32],
    seed_row: RowId,
) -> GreedyRowResult<GreedyRectangle> {
    let mut best_rectangle = GreedyRectangle::default();
    if matrix.is_empty() {
        return Ok(best_rectangle);
    }

    let seed = matrix
        .row(seed_row)
        .ok_or(GreedyRowError::SeedRowNotFound(seed_row))?;
    let mut rectangle = GreedyRectangle::default();
    rectangle.rows.insert(seed_row);
    for element in seed.elements() {
        rectangle.columns.insert(element.column);
    }

    let mut column_values = BTreeMap::new();
    for element in seed.elements() {
        let column_cost = cost_for_column(column_costs, element.column)?;
        column_values.insert(element.column, element.value - column_cost);
    }

    let mut total_row_cost = cost_for_row(row_costs, seed_row)?;
    rectangle.value = value_for_rectangle(&rectangle, total_row_cost, &column_values);

    let mut next_row = find_max_row_intersection(matrix, row_costs, &rectangle, &column_values)?;
    while let Some(row) = next_row {
        if rectangle.column_len() <= 1 {
            break;
        }

        rectangle.rows.insert(row.index());
        total_row_cost += cost_for_row(row_costs, row.index())?;
        rectangle.columns = rectangle
            .columns
            .iter()
            .copied()
            .filter(|column| row.contains(*column))
            .collect();

        for element in row.elements() {
            *column_values.entry(element.column).or_default() += element.value;
        }

        rectangle.value = value_for_rectangle(&rectangle, total_row_cost, &column_values);
        debug_assert_eq!(
            check_rectangle_value(matrix, row_costs, column_costs, &rectangle)?,
            rectangle.value
        );

        if rectangle.value > best_rectangle.value {
            best_rectangle = rectangle.clone();
        }

        next_row = find_max_row_intersection(matrix, row_costs, &rectangle, &column_values)?;
    }

    Ok(best_rectangle)
}

pub fn check_rectangle_value(
    matrix: &WeightedMatrix,
    row_costs: &[i32],
    column_costs: &[i32],
    rectangle: &GreedyRectangle,
) -> GreedyRowResult<i32> {
    let mut value = 0;

    for row in &rectangle.rows {
        for column in &rectangle.columns {
            let element =
                matrix
                    .find(*row, *column)
                    .ok_or(GreedyRowError::MatrixMissingElement {
                        row: *row,
                        column: *column,
                    })?;
            value += element.value;
        }
    }

    for row in &rectangle.rows {
        value -= cost_for_row(row_costs, *row)?;
    }

    for column in &rectangle.columns {
        value -= cost_for_column(column_costs, *column)?;
    }

    Ok(value)
}

fn find_max_row_intersection<'a>(
    matrix: &'a WeightedMatrix,
    row_costs: &[i32],
    rectangle: &GreedyRectangle,
    column_values: &BTreeMap<ColumnId, i32>,
) -> GreedyRowResult<Option<WeightedRow<'a>>> {
    if matrix.is_empty() {
        return Ok(None);
    }

    let mut row_values = BTreeMap::<RowId, i32>::new();
    for column in &rectangle.columns {
        let Some(matrix_column) = matrix.column(*column) else {
            continue;
        };

        for element in matrix_column.elements() {
            row_values.insert(element.row, -cost_for_row(row_costs, element.row)?);
        }
    }

    for column in &rectangle.columns {
        let Some(matrix_column) = matrix.column(*column) else {
            continue;
        };
        let current_column_value = *column_values.get(column).unwrap_or(&0);

        for element in matrix_column.elements() {
            *row_values.entry(element.row).or_default() += element.value + current_column_value;
        }
    }

    let mut max_value = 0;
    let mut best_row = None;
    for column in &rectangle.columns {
        let Some(matrix_column) = matrix.column(*column) else {
            continue;
        };

        for element in matrix_column.elements() {
            let value = *row_values.get(&element.row).unwrap_or(&0);
            if value > max_value && !rectangle.rows.contains(&element.row) {
                max_value = value;
                best_row = matrix.row(element.row);
            }
        }
    }

    Ok(best_row)
}

fn value_for_rectangle(
    rectangle: &GreedyRectangle,
    total_row_cost: i32,
    column_values: &BTreeMap<ColumnId, i32>,
) -> i32 {
    rectangle
        .columns
        .iter()
        .map(|column| *column_values.get(column).unwrap_or(&0))
        .sum::<i32>()
        - total_row_cost
}

fn cost_for_row(row_costs: &[i32], row: RowId) -> GreedyRowResult<i32> {
    row_costs
        .get(row.0)
        .copied()
        .ok_or(GreedyRowError::MissingRowCost(row))
}

fn cost_for_column(column_costs: &[i32], column: ColumnId) -> GreedyRowResult<i32> {
    column_costs
        .get(column.0)
        .copied()
        .ok_or(GreedyRowError::MissingColumnCost(column))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greedy_row_returns_empty_rectangle_for_empty_matrix() {
        let rectangle = greedy_row(&WeightedMatrix::new(), &[], &[], RowId(0)).unwrap();

        assert_eq!(rectangle, GreedyRectangle::default());
    }

    #[test]
    fn greedy_row_finds_best_positive_row_growth() {
        let mut matrix = WeightedMatrix::new();
        matrix.insert(RowId(0), ColumnId(0), 4);
        matrix.insert(RowId(0), ColumnId(1), 3);
        matrix.insert(RowId(0), ColumnId(2), 1);
        matrix.insert(RowId(1), ColumnId(0), 6);
        matrix.insert(RowId(1), ColumnId(1), 2);
        matrix.insert(RowId(2), ColumnId(1), 7);
        matrix.insert(RowId(2), ColumnId(2), 6);

        let rectangle = greedy_row(&matrix, &[2, 1, 1], &[1, 1, 1], RowId(0)).unwrap();

        assert_eq!(rectangle.rows(), &set_rows(&[0, 2]));
        assert_eq!(rectangle.columns(), &set_columns(&[1, 2]));
        assert_eq!(rectangle.value(), 12);
        assert_eq!(
            check_rectangle_value(&matrix, &[2, 1, 1], &[1, 1, 1], &rectangle).unwrap(),
            rectangle.value()
        );
    }

    #[test]
    fn greedy_row_drops_to_empty_when_no_positive_expansion_beats_zero() {
        let mut matrix = WeightedMatrix::new();
        matrix.insert(RowId(0), ColumnId(0), 1);
        matrix.insert(RowId(0), ColumnId(1), 1);
        matrix.insert(RowId(1), ColumnId(0), 1);
        matrix.insert(RowId(1), ColumnId(1), 1);

        let rectangle = greedy_row(&matrix, &[10, 10], &[3, 3], RowId(0)).unwrap();

        assert_eq!(rectangle, GreedyRectangle::default());
    }

    #[test]
    fn greedy_row_stops_before_collapsing_to_single_column() {
        let mut matrix = WeightedMatrix::new();
        matrix.insert(RowId(0), ColumnId(0), 3);
        matrix.insert(RowId(0), ColumnId(1), 3);
        matrix.insert(RowId(1), ColumnId(0), 8);

        let rectangle = greedy_row(&matrix, &[1, 1], &[0, 0], RowId(0)).unwrap();

        assert_eq!(rectangle.rows(), &set_rows(&[0, 1]));
        assert_eq!(rectangle.columns(), &set_columns(&[0]));
        assert_eq!(rectangle.value(), 9);
    }

    #[test]
    fn missing_costs_and_seed_row_are_reported() {
        let mut matrix = WeightedMatrix::new();
        matrix.insert(RowId(0), ColumnId(3), 5);

        assert_eq!(
            greedy_row(&matrix, &[], &[0, 0, 0, 0], RowId(0)),
            Err(GreedyRowError::MissingRowCost(RowId(0)))
        );
        assert_eq!(
            greedy_row(&matrix, &[0], &[0], RowId(0)),
            Err(GreedyRowError::MissingColumnCost(ColumnId(3)))
        );
        assert_eq!(
            greedy_row(&matrix, &[0], &[0, 0, 0, 0], RowId(2)),
            Err(GreedyRowError::SeedRowNotFound(RowId(2)))
        );
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_metadata_tokens_are_present_in_this_port() {
        let source = include_str!("greedyrow.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday", "1-")));
    }

    fn set_rows(rows: &[usize]) -> BTreeSet<RowId> {
        rows.iter().copied().map(RowId).collect()
    }

    fn set_columns(columns: &[usize]) -> BTreeSet<ColumnId> {
        columns.iter().copied().map(ColumnId).collect()
    }
}
