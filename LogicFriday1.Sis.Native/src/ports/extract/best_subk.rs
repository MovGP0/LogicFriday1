//! Native Rust best subkernel selection for SIS extraction.
//!
//! The selector evaluates every prime rectangle in the kernel-cube matrix and
//! every single-row rectangle, then returns the highest-valued rectangle for the
//! requested cost model. Ping-pong selection is deliberately left as a separate
//! native dependency because that algorithm lives in its own extraction unit.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SisIndex(pub i32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelCubeCell {
    pub value: i32,
    pub sis_index: SisIndex,
}

impl KernelCubeCell {
    pub fn new(value: i32, sis_index: SisIndex) -> Self {
        Self { value, sis_index }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KernelCubeMatrix {
    rows: BTreeMap<usize, BTreeMap<usize, KernelCubeCell>>,
    columns: BTreeMap<usize, BTreeSet<usize>>,
}

impl KernelCubeMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &mut self,
        row: usize,
        column: usize,
        cell: KernelCubeCell,
    ) -> Option<KernelCubeCell> {
        let old = self.rows.entry(row).or_default().insert(column, cell);
        self.columns.entry(column).or_default().insert(row);
        old
    }

    pub fn cell(&self, row: usize, column: usize) -> Option<&KernelCubeCell> {
        self.rows.get(&row).and_then(|columns| columns.get(&column))
    }

    pub fn row(&self, row: usize) -> Option<&BTreeMap<usize, KernelCubeCell>> {
        self.rows.get(&row)
    }

    pub fn row_indexes(&self) -> impl Iterator<Item = usize> + '_ {
        self.rows.keys().copied()
    }

    pub fn column_indexes(&self) -> impl Iterator<Item = usize> + '_ {
        self.columns.keys().copied()
    }

    pub fn column_rows(&self, column: usize) -> Option<&BTreeSet<usize>> {
        self.columns.get(&column)
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubkernelOption {
    PingPong,
    SumOfProducts,
    Factored,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SubkernelRectangle {
    rows: BTreeSet<usize>,
    columns: BTreeSet<usize>,
    value: i32,
}

impl SubkernelRectangle {
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
            value: 0,
        }
    }

    pub fn rows(&self) -> &BTreeSet<usize> {
        &self.rows
    }

    pub fn columns(&self) -> &BTreeSet<usize> {
        &self.columns
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() || self.columns.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BestSubkernelError {
    PingPongUnavailable,
    MissingRowCost { row: usize },
    MissingColumnCost { column: usize },
    MissingMatrixCell { row: usize, column: usize },
}

impl fmt::Display for BestSubkernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PingPongUnavailable => {
                write!(f, "ping-pong subkernel selection has not been ported yet")
            }
            Self::MissingRowCost { row } => write!(f, "missing row cost for row {row}"),
            Self::MissingColumnCost { column } => {
                write!(f, "missing column cost for column {column}")
            }
            Self::MissingMatrixCell { row, column } => {
                write!(f, "missing kernel-cube matrix cell ({row}, {column})")
            }
        }
    }
}

impl Error for BestSubkernelError {}

pub type BestSubkernelResult<T> = Result<T, BestSubkernelError>;

pub fn choose_subkernel(
    matrix: &KernelCubeMatrix,
    row_costs: &[i32],
    column_costs: &[i32],
    option: SubkernelOption,
) -> BestSubkernelResult<SubkernelRectangle> {
    match option {
        SubkernelOption::PingPong => Err(BestSubkernelError::PingPongUnavailable),
        SubkernelOption::SumOfProducts => {
            best_subkernel(matrix, row_costs, column_costs, score_sum_of_products)
        }
        SubkernelOption::Factored => {
            best_subkernel(matrix, row_costs, column_costs, score_factored)
        }
    }
}

pub fn score_sum_of_products(
    matrix: &KernelCubeMatrix,
    row_costs: &[i32],
    column_costs: &[i32],
    rectangle: &SubkernelRectangle,
) -> BestSubkernelResult<i32> {
    if rectangle.is_empty() {
        return Ok(0);
    }

    let mut co_kernel_literals = 0;
    for row in &rectangle.rows {
        co_kernel_literals += cost_for_row(row_costs, *row)? - 1;
    }

    let mut kernel_literals = 0;
    for column in &rectangle.columns {
        kernel_literals += cost_for_column(column_costs, *column)?;
    }

    let mut covered_value = 0;
    for row in &rectangle.rows {
        for column in &rectangle.columns {
            covered_value += matrix
                .cell(*row, *column)
                .ok_or(BestSubkernelError::MissingMatrixCell {
                    row: *row,
                    column: *column,
                })?
                .value;
        }
    }

    Ok(covered_value - kernel_literals - co_kernel_literals - rectangle.rows.len() as i32)
}

pub fn score_factored(
    matrix: &KernelCubeMatrix,
    _row_costs: &[i32],
    column_costs: &[i32],
    rectangle: &SubkernelRectangle,
) -> BestSubkernelResult<i32> {
    if rectangle.is_empty() {
        return Ok(0);
    }

    let mut kernel_literals = 0;
    for column in &rectangle.columns {
        kernel_literals += cost_for_column(column_costs, *column)?;
    }

    let mut fanouts = BTreeSet::new();
    for row in &rectangle.rows {
        for column in &rectangle.columns {
            let cell = matrix
                .cell(*row, *column)
                .ok_or(BestSubkernelError::MissingMatrixCell {
                    row: *row,
                    column: *column,
                })?;
            fanouts.insert(cell.sis_index);
        }
    }

    Ok((fanouts.len() as i32 - 1) * (kernel_literals - 1) - 1)
}

fn best_subkernel(
    matrix: &KernelCubeMatrix,
    row_costs: &[i32],
    column_costs: &[i32],
    score: fn(&KernelCubeMatrix, &[i32], &[i32], &SubkernelRectangle) -> BestSubkernelResult<i32>,
) -> BestSubkernelResult<SubkernelRectangle> {
    let mut best = None;

    for mut rectangle in generate_prime_rectangles(matrix) {
        rectangle.value = score(matrix, row_costs, column_costs, &rectangle)?;
        update_best(&mut best, rectangle);
    }

    for row in matrix.row_indexes() {
        let columns = matrix
            .row(row)
            .expect("row index came from the matrix")
            .keys()
            .copied();
        let mut rectangle = SubkernelRectangle::from_rows_columns([row], columns);
        rectangle.value = score(matrix, row_costs, column_costs, &rectangle)?;
        update_best(&mut best, rectangle);
    }

    Ok(best.unwrap_or_default())
}

fn update_best(best: &mut Option<SubkernelRectangle>, rectangle: SubkernelRectangle) {
    if best
        .as_ref()
        .is_none_or(|best_rectangle| rectangle.value > best_rectangle.value)
    {
        *best = Some(rectangle);
    }
}

fn generate_prime_rectangles(matrix: &KernelCubeMatrix) -> Vec<SubkernelRectangle> {
    let mut rectangles = Vec::new();
    let rectangle = SubkernelRectangle::new();
    generate_prime_rectangles_from(matrix, &rectangle, 0, &mut rectangles);

    if !has_full_column(matrix) {
        rectangles.push(rectangle);
    }

    rectangles
}

fn generate_prime_rectangles_from(
    matrix: &KernelCubeMatrix,
    rectangle: &SubkernelRectangle,
    min_column: usize,
    rectangles: &mut Vec<SubkernelRectangle>,
) {
    let columns = matrix.column_indexes().collect::<Vec<_>>();

    for column in columns {
        let Some(rows) = matrix.column_rows(column) else {
            continue;
        };

        if rows.len() < 2 || column < min_column {
            continue;
        }

        let mut submatrix = rows_induced_by_column(matrix, rows);
        let mut subrectangle = SubkernelRectangle {
            rows: rows.iter().copied().collect(),
            columns: rectangle.columns.clone(),
            value: 0,
        };

        let mut already_generated = false;
        let full_columns = submatrix.column_indexes().collect::<Vec<_>>();
        for full_column in full_columns {
            let full_column_len = submatrix
                .column_rows(full_column)
                .expect("column index came from the submatrix")
                .len();
            if full_column_len == rows.len() {
                if full_column < column {
                    already_generated = true;
                    break;
                }

                subrectangle.columns.insert(full_column);
                delete_column(&mut submatrix, full_column);
            }
        }

        if !already_generated {
            rectangles.push(subrectangle.clone());
            generate_prime_rectangles_from(&submatrix, &subrectangle, column, rectangles);
        }
    }
}

fn rows_induced_by_column(matrix: &KernelCubeMatrix, rows: &BTreeSet<usize>) -> KernelCubeMatrix {
    let mut submatrix = KernelCubeMatrix::new();
    for row in rows {
        if let Some(columns) = matrix.row(*row) {
            for (column, cell) in columns {
                submatrix.insert(*row, *column, *cell);
            }
        }
    }

    submatrix
}

fn delete_column(matrix: &mut KernelCubeMatrix, column: usize) {
    let Some(rows) = matrix.columns.remove(&column) else {
        return;
    };

    for row in rows {
        let should_remove_row = if let Some(columns) = matrix.rows.get_mut(&row) {
            columns.remove(&column);
            columns.is_empty()
        } else {
            false
        };

        if should_remove_row {
            matrix.rows.remove(&row);
        }
    }
}

fn has_full_column(matrix: &KernelCubeMatrix) -> bool {
    matrix
        .columns
        .values()
        .any(|rows| rows.len() == matrix.row_count())
}

fn cost_for_row(row_costs: &[i32], row: usize) -> BestSubkernelResult<i32> {
    row_costs
        .get(row)
        .copied()
        .ok_or(BestSubkernelError::MissingRowCost { row })
}

fn cost_for_column(column_costs: &[i32], column: usize) -> BestSubkernelResult<i32> {
    column_costs
        .get(column)
        .copied()
        .ok_or(BestSubkernelError::MissingColumnCost { column })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(value: i32, sis_index: i32) -> KernelCubeCell {
        KernelCubeCell::new(value, SisIndex(sis_index))
    }

    fn sample_matrix() -> KernelCubeMatrix {
        let mut matrix = KernelCubeMatrix::new();
        matrix.insert(0, 0, cell(4, 10));
        matrix.insert(0, 1, cell(5, 10));
        matrix.insert(0, 2, cell(3, 11));
        matrix.insert(1, 0, cell(6, 12));
        matrix.insert(1, 1, cell(7, 12));
        matrix.insert(2, 1, cell(2, 13));
        matrix.insert(2, 2, cell(9, 13));
        matrix
    }

    #[test]
    fn sop_selection_scores_prime_rectangles_and_single_rows() {
        let matrix = sample_matrix();

        let rectangle = choose_subkernel(
            &matrix,
            &[2, 3, 2],
            &[2, 1, 2],
            SubkernelOption::SumOfProducts,
        )
        .unwrap();

        assert_eq!(rectangle.rows(), &BTreeSet::from([0, 1]));
        assert_eq!(rectangle.columns(), &BTreeSet::from([0, 1]));
        assert_eq!(rectangle.value(), 14);
    }

    #[test]
    fn single_row_rectangles_are_considered_after_prime_rectangles() {
        let mut matrix = KernelCubeMatrix::new();
        matrix.insert(0, 0, cell(10, 1));
        matrix.insert(0, 1, cell(10, 1));
        matrix.insert(1, 0, cell(1, 2));
        matrix.insert(1, 1, cell(1, 2));

        let rectangle =
            choose_subkernel(&matrix, &[1, 10], &[1, 1], SubkernelOption::SumOfProducts).unwrap();

        assert_eq!(rectangle.rows(), &BTreeSet::from([0]));
        assert_eq!(rectangle.columns(), &BTreeSet::from([0, 1]));
        assert_eq!(rectangle.value(), 17);
    }

    #[test]
    fn factored_selection_counts_unique_fanouts() {
        let matrix = sample_matrix();

        let rectangle =
            choose_subkernel(&matrix, &[0, 0, 0], &[2, 3, 4], SubkernelOption::Factored).unwrap();

        assert_eq!(rectangle.rows(), &BTreeSet::from([0, 2]));
        assert_eq!(rectangle.columns(), &BTreeSet::from([1, 2]));
        assert_eq!(rectangle.value(), 11);
    }

    #[test]
    fn empty_matrix_returns_an_empty_rectangle_for_native_cost_models() {
        let matrix = KernelCubeMatrix::new();

        assert_eq!(
            choose_subkernel(&matrix, &[], &[], SubkernelOption::SumOfProducts),
            Ok(SubkernelRectangle::new())
        );
        assert_eq!(
            choose_subkernel(&matrix, &[], &[], SubkernelOption::Factored),
            Ok(SubkernelRectangle::new())
        );
    }

    #[test]
    fn missing_costs_are_reported() {
        let matrix = sample_matrix();

        assert_eq!(
            choose_subkernel(&matrix, &[], &[1, 1, 1], SubkernelOption::SumOfProducts),
            Err(BestSubkernelError::MissingRowCost { row: 0 })
        );
        assert_eq!(
            choose_subkernel(&matrix, &[1, 1, 1], &[], SubkernelOption::Factored),
            Err(BestSubkernelError::MissingColumnCost { column: 0 })
        );
    }

    #[test]
    fn ping_pong_selection_reports_the_unported_native_dependency() {
        let matrix = KernelCubeMatrix::new();

        assert_eq!(
            choose_subkernel(&matrix, &[], &[], SubkernelOption::PingPong),
            Err(BestSubkernelError::PingPongUnavailable)
        );
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_metadata_tokens_are_present_in_this_port() {
        let source = include_str!("best_subk.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday", "1-", "8j8")));
    }
}
