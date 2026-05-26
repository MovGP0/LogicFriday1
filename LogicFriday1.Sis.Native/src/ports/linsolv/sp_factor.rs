use std::collections::HashMap;
use std::error::Error;
use std::fmt;

const DEFAULT_RELATIVE_THRESHOLD: f64 = 1.0e-3;
const DEFAULT_ABSOLUTE_THRESHOLD: f64 = 0.0;
const ZERO_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PartitionMode {
    Default,
    Direct,
    Indirect,
    Auto,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PartitionAddressing {
    Direct,
    Indirect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PivotSelectionMethod {
    Singleton,
    Diagonal,
    EntireMatrix,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactorWarning {
    SmallPivot,
}

#[derive(Clone, Copy, Debug)]
pub struct PivotOptions {
    pub relative_threshold: f64,
    pub absolute_threshold: f64,
    pub diagonal_pivoting: bool,
}

impl Default for PivotOptions {
    fn default() -> Self {
        Self {
            relative_threshold: DEFAULT_RELATIVE_THRESHOLD,
            absolute_threshold: DEFAULT_ABSOLUTE_THRESHOLD,
            diagonal_pivoting: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PivotRecord {
    pub step: usize,
    pub row: usize,
    pub col: usize,
    pub value: f64,
    pub method: PivotSelectionMethod,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FactorizationReport {
    pub row_permutation: Vec<usize>,
    pub col_permutation: Vec<usize>,
    pub pivots: Vec<PivotRecord>,
    pub fillins: usize,
    pub warning: Option<FactorWarning>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SparseFactorError {
    InvalidSize,
    IndexOutOfRange { row: usize, col: usize, size: usize },
    AlreadyFactored,
    Singular { row: usize, col: usize },
    ZeroDiagonal { index: usize },
}

impl fmt::Display for SparseFactorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize => write!(f, "matrix size must be greater than zero"),
            Self::IndexOutOfRange { row, col, size } => {
                write!(f, "matrix index ({row}, {col}) is outside 1..={size}")
            }
            Self::AlreadyFactored => write!(f, "matrix is already factored"),
            Self::Singular { row, col } => {
                write!(f, "matrix is singular at external row {row}, column {col}")
            }
            Self::ZeroDiagonal { index } => {
                write!(f, "zero diagonal encountered at external index {index}")
            }
        }
    }
}

impl Error for SparseFactorError {}

pub type SparseFactorResult<T> = Result<T, SparseFactorError>;

#[derive(Clone, Debug)]
pub struct SparseFactorMatrix {
    size: usize,
    values: HashMap<(usize, usize), f64>,
    lu: Vec<Vec<f64>>,
    row_permutation: Vec<usize>,
    col_permutation: Vec<usize>,
    partition: Vec<PartitionAddressing>,
    factored: bool,
    needs_ordering: bool,
    rel_threshold: f64,
    abs_threshold: f64,
    last_report: Option<FactorizationReport>,
}

impl SparseFactorMatrix {
    pub fn new(size: usize) -> SparseFactorResult<Self> {
        if size == 0 {
            return Err(SparseFactorError::InvalidSize);
        }

        Ok(Self {
            size,
            values: HashMap::new(),
            lu: vec![vec![0.0; size]; size],
            row_permutation: one_based_identity(size),
            col_permutation: one_based_identity(size),
            partition: vec![PartitionAddressing::Indirect; size],
            factored: false,
            needs_ordering: true,
            rel_threshold: DEFAULT_RELATIVE_THRESHOLD,
            abs_threshold: DEFAULT_ABSOLUTE_THRESHOLD,
            last_report: None,
        })
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn set(&mut self, row: usize, col: usize, value: f64) -> SparseFactorResult<()> {
        self.check_index(row, col)?;
        if value.abs() <= ZERO_TOLERANCE {
            self.values.remove(&(row, col));
        } else {
            self.values.insert((row, col), value);
        }

        self.factored = false;
        self.needs_ordering = true;
        self.last_report = None;
        Ok(())
    }

    pub fn add(&mut self, row: usize, col: usize, value: f64) -> SparseFactorResult<()> {
        self.check_index(row, col)?;
        let next = self.values.get(&(row, col)).copied().unwrap_or(0.0) + value;
        self.set(row, col, next)
    }

    pub fn get(&self, row: usize, col: usize) -> SparseFactorResult<f64> {
        self.check_index(row, col)?;
        Ok(self.values.get(&(row, col)).copied().unwrap_or(0.0))
    }

    pub fn clear_values(&mut self) {
        self.values.clear();
        self.lu = vec![vec![0.0; self.size]; self.size];
        self.factored = false;
        self.needs_ordering = true;
        self.last_report = None;
    }

    pub fn is_factored(&self) -> bool {
        self.factored
    }

    pub fn row_permutation(&self) -> &[usize] {
        &self.row_permutation
    }

    pub fn col_permutation(&self) -> &[usize] {
        &self.col_permutation
    }

    pub fn partition(&self) -> &[PartitionAddressing] {
        &self.partition
    }

    pub fn lower(&self) -> SparseFactorResult<Vec<Vec<f64>>> {
        self.require_factored()?;
        let mut lower = vec![vec![0.0; self.size]; self.size];
        for row in 0..self.size {
            lower[row][row] = 1.0;
            for col in 0..row {
                lower[row][col] = self.lu[row][col];
            }
        }
        Ok(lower)
    }

    pub fn upper(&self) -> SparseFactorResult<Vec<Vec<f64>>> {
        self.require_factored()?;
        let mut upper = vec![vec![0.0; self.size]; self.size];
        for row in 0..self.size {
            for col in row..self.size {
                upper[row][col] = self.lu[row][col];
            }
        }
        Ok(upper)
    }

    pub fn last_report(&self) -> Option<&FactorizationReport> {
        self.last_report.as_ref()
    }

    pub fn partition_matrix(&mut self, mode: PartitionMode) -> Vec<PartitionAddressing> {
        let actual = if mode == PartitionMode::Default {
            PartitionMode::Auto
        } else {
            mode
        };

        self.partition = match actual {
            PartitionMode::Direct => vec![PartitionAddressing::Direct; self.size],
            PartitionMode::Indirect => vec![PartitionAddressing::Indirect; self.size],
            PartitionMode::Auto | PartitionMode::Default => {
                let dense_limit = self.size.saturating_div(3).max(1);
                (0..self.size)
                    .map(|col| {
                        if self.column_nonzero_count(col + 1) > dense_limit {
                            PartitionAddressing::Direct
                        } else {
                            PartitionAddressing::Indirect
                        }
                    })
                    .collect()
            }
        };
        self.partition.clone()
    }

    pub fn order_and_factor(
        &mut self,
        rhs: Option<&[f64]>,
        options: PivotOptions,
    ) -> SparseFactorResult<FactorizationReport> {
        if self.factored {
            return Err(SparseFactorError::AlreadyFactored);
        }

        let mut dense = self.to_dense();
        let mut rows = one_based_identity(self.size);
        let mut cols = one_based_identity(self.size);
        let rel_threshold =
            normalized_relative_threshold(options.relative_threshold, self.rel_threshold);
        let abs_threshold = if options.absolute_threshold < 0.0 {
            self.abs_threshold
        } else {
            options.absolute_threshold
        };

        self.rel_threshold = rel_threshold;
        self.abs_threshold = abs_threshold;
        self.partition_matrix(PartitionMode::Auto);

        let mut warning = None;
        let mut fillins = 0;
        let mut pivots = Vec::with_capacity(self.size);

        for step in 0..self.size {
            let pivot = self
                .search_for_pivot(
                    &dense,
                    &rows,
                    &cols,
                    rhs,
                    step,
                    rel_threshold,
                    abs_threshold,
                    options.diagonal_pivoting,
                )
                .ok_or_else(|| SparseFactorError::Singular {
                    row: rows[step],
                    col: cols[step],
                })?;

            if pivot.small {
                warning = Some(FactorWarning::SmallPivot);
            }

            dense.swap(step, pivot.row);
            rows.swap(step, pivot.row);
            for row in &mut dense {
                row.swap(step, pivot.col);
            }
            cols.swap(step, pivot.col);

            let pivot_value = dense[step][step];
            if pivot_value.abs() <= ZERO_TOLERANCE {
                return Err(SparseFactorError::Singular {
                    row: rows[step],
                    col: cols[step],
                });
            }

            pivots.push(PivotRecord {
                step: step + 1,
                row: rows[step],
                col: cols[step],
                value: pivot_value,
                method: pivot.method,
            });

            fillins += eliminate_step(&mut dense, step, pivot_value);
        }

        self.lu = dense;
        self.row_permutation = rows;
        self.col_permutation = cols;
        self.factored = true;
        self.needs_ordering = false;

        let report = FactorizationReport {
            row_permutation: self.row_permutation.clone(),
            col_permutation: self.col_permutation.clone(),
            pivots,
            fillins,
            warning,
        };
        self.last_report = Some(report.clone());
        Ok(report)
    }

    pub fn factor(&mut self) -> SparseFactorResult<FactorizationReport> {
        if self.factored {
            return Err(SparseFactorError::AlreadyFactored);
        }

        if self.needs_ordering {
            return self.order_and_factor(None, PivotOptions::default());
        }

        if self.partition.is_empty() {
            self.partition_matrix(PartitionMode::Default);
        }

        let mut dense = self.to_permuted_dense();
        let mut fillins = 0;
        let mut pivots = Vec::with_capacity(self.size);

        for step in 0..self.size {
            let pivot_value = dense[step][step];
            if pivot_value.abs() <= ZERO_TOLERANCE {
                return Err(SparseFactorError::ZeroDiagonal {
                    index: self.row_permutation[step],
                });
            }

            pivots.push(PivotRecord {
                step: step + 1,
                row: self.row_permutation[step],
                col: self.col_permutation[step],
                value: pivot_value,
                method: PivotSelectionMethod::Diagonal,
            });
            fillins += eliminate_step(&mut dense, step, pivot_value);
        }

        self.lu = dense;
        self.factored = true;
        let report = FactorizationReport {
            row_permutation: self.row_permutation.clone(),
            col_permutation: self.col_permutation.clone(),
            pivots,
            fillins,
            warning: None,
        };
        self.last_report = Some(report.clone());
        Ok(report)
    }

    pub fn refactor_with_current_order(&mut self) -> SparseFactorResult<FactorizationReport> {
        self.factored = false;
        self.needs_ordering = false;
        self.factor()
    }

    pub fn solve(&self, rhs: &[f64]) -> SparseFactorResult<Vec<f64>> {
        self.require_factored()?;
        if rhs.len() != self.size {
            return Err(SparseFactorError::IndexOutOfRange {
                row: rhs.len(),
                col: 1,
                size: self.size,
            });
        }

        let mut y = vec![0.0; self.size];
        for row in 0..self.size {
            let external_row = self.row_permutation[row] - 1;
            let mut value = rhs[external_row];
            for (col, y_col) in y.iter().enumerate().take(row) {
                value -= self.lu[row][col] * y_col;
            }
            y[row] = value;
        }

        let mut z = vec![0.0; self.size];
        for row in (0..self.size).rev() {
            let mut value = y[row];
            for (col, z_col) in z.iter().enumerate().skip(row + 1) {
                value -= self.lu[row][col] * z_col;
            }
            z[row] = value / self.lu[row][row];
        }

        let mut solution = vec![0.0; self.size];
        for (internal_col, external_col) in self.col_permutation.iter().enumerate() {
            solution[*external_col - 1] = z[internal_col];
        }
        Ok(solution)
    }

    fn search_for_pivot(
        &self,
        dense: &[Vec<f64>],
        rows: &[usize],
        cols: &[usize],
        rhs: Option<&[f64]>,
        step: usize,
        rel_threshold: f64,
        abs_threshold: f64,
        diagonal_pivoting: bool,
    ) -> Option<PivotCandidate> {
        if let Some(pivot) =
            search_singleton(dense, rows, cols, rhs, step, rel_threshold, abs_threshold)
        {
            return Some(pivot);
        }

        if diagonal_pivoting {
            if let Some(pivot) = search_diagonal(dense, step, rel_threshold, abs_threshold) {
                return Some(pivot);
            }
        }

        search_entire_matrix(dense, step, rel_threshold, abs_threshold)
    }

    fn to_dense(&self) -> Vec<Vec<f64>> {
        let mut dense = vec![vec![0.0; self.size]; self.size];
        for (&(row, col), &value) in &self.values {
            dense[row - 1][col - 1] = value;
        }
        dense
    }

    fn to_permuted_dense(&self) -> Vec<Vec<f64>> {
        let mut dense = vec![vec![0.0; self.size]; self.size];
        for (internal_row, external_row) in self.row_permutation.iter().enumerate() {
            for (internal_col, external_col) in self.col_permutation.iter().enumerate() {
                dense[internal_row][internal_col] = self
                    .values
                    .get(&(*external_row, *external_col))
                    .copied()
                    .unwrap_or(0.0);
            }
        }
        dense
    }

    fn column_nonzero_count(&self, col: usize) -> usize {
        self.values
            .keys()
            .filter(|(_, candidate_col)| *candidate_col == col)
            .count()
    }

    fn check_index(&self, row: usize, col: usize) -> SparseFactorResult<()> {
        if row == 0 || col == 0 || row > self.size || col > self.size {
            return Err(SparseFactorError::IndexOutOfRange {
                row,
                col,
                size: self.size,
            });
        }
        Ok(())
    }

    fn require_factored(&self) -> SparseFactorResult<()> {
        if self.factored {
            Ok(())
        } else {
            Err(SparseFactorError::ZeroDiagonal { index: 0 })
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PivotCandidate {
    row: usize,
    col: usize,
    value: f64,
    product: usize,
    method: PivotSelectionMethod,
    small: bool,
}

pub fn order_and_factor(
    matrix: &mut SparseFactorMatrix,
    rhs: Option<&[f64]>,
    options: PivotOptions,
) -> SparseFactorResult<FactorizationReport> {
    matrix.order_and_factor(rhs, options)
}

pub fn factor(matrix: &mut SparseFactorMatrix) -> SparseFactorResult<FactorizationReport> {
    matrix.factor()
}

pub fn partition(matrix: &mut SparseFactorMatrix, mode: PartitionMode) -> Vec<PartitionAddressing> {
    matrix.partition_matrix(mode)
}

fn normalized_relative_threshold(candidate: f64, current: f64) -> f64 {
    if candidate <= 0.0 || candidate > 1.0 {
        current
    } else {
        candidate
    }
}

fn one_based_identity(size: usize) -> Vec<usize> {
    (1..=size).collect()
}

fn eliminate_step(dense: &mut [Vec<f64>], step: usize, pivot_value: f64) -> usize {
    let mut fillins = 0;
    let size = dense.len();
    for row in (step + 1)..size {
        if dense[row][step].abs() <= ZERO_TOLERANCE {
            continue;
        }

        let multiplier = dense[row][step] / pivot_value;
        dense[row][step] = multiplier;
        for col in (step + 1)..size {
            if dense[step][col].abs() <= ZERO_TOLERANCE {
                continue;
            }

            let was_zero = dense[row][col].abs() <= ZERO_TOLERANCE;
            dense[row][col] -= multiplier * dense[step][col];
            if was_zero && dense[row][col].abs() > ZERO_TOLERANCE {
                fillins += 1;
            }
            if dense[row][col].abs() <= ZERO_TOLERANCE {
                dense[row][col] = 0.0;
            }
        }
    }
    fillins
}

fn search_singleton(
    dense: &[Vec<f64>],
    rows: &[usize],
    cols: &[usize],
    rhs: Option<&[f64]>,
    step: usize,
    rel_threshold: f64,
    abs_threshold: f64,
) -> Option<PivotCandidate> {
    let size = dense.len();
    for index in step..size {
        let row_count = active_row_count(dense, index, step);
        let col_count = active_col_count(dense, index, step);
        if row_count != 1 && col_count != 1 {
            continue;
        }

        if rhs_sparse_excludes(rhs, rows[index]) {
            continue;
        }

        let mut best = None;
        if row_count == 1 {
            for col in step..size {
                if dense[index][col].abs() > ZERO_TOLERANCE {
                    best = Some((index, col));
                    break;
                }
            }
        } else {
            for row in step..size {
                if dense[row][index].abs() > ZERO_TOLERANCE {
                    best = Some((row, index));
                    break;
                }
            }
        }

        if let Some((row, col)) = best {
            let candidate = make_candidate(
                dense,
                row,
                col,
                step,
                rel_threshold,
                abs_threshold,
                PivotSelectionMethod::Singleton,
            );
            if candidate_allowed(
                &candidate,
                dense,
                row,
                col,
                step,
                rel_threshold,
                abs_threshold,
            ) {
                let _ = cols;
                return Some(candidate);
            }
        }
    }
    None
}

fn search_diagonal(
    dense: &[Vec<f64>],
    step: usize,
    rel_threshold: f64,
    abs_threshold: f64,
) -> Option<PivotCandidate> {
    let size = dense.len();
    let mut best: Option<PivotCandidate> = None;
    for index in step..size {
        let candidate = make_candidate(
            dense,
            index,
            index,
            step,
            rel_threshold,
            abs_threshold,
            PivotSelectionMethod::Diagonal,
        );
        if !candidate_allowed(
            &candidate,
            dense,
            index,
            index,
            step,
            rel_threshold,
            abs_threshold,
        ) {
            continue;
        }
        best = better_pivot(best, candidate);
    }
    best
}

fn search_entire_matrix(
    dense: &[Vec<f64>],
    step: usize,
    rel_threshold: f64,
    abs_threshold: f64,
) -> Option<PivotCandidate> {
    let size = dense.len();
    let mut best: Option<PivotCandidate> = None;
    let mut largest_fallback: Option<PivotCandidate> = None;
    for row in step..size {
        for col in step..size {
            if dense[row][col].abs() <= ZERO_TOLERANCE {
                continue;
            }

            let candidate = make_candidate(
                dense,
                row,
                col,
                step,
                rel_threshold,
                abs_threshold,
                PivotSelectionMethod::EntireMatrix,
            );
            if candidate.value.abs() > abs_threshold
                && candidate.value.abs() >= rel_threshold * largest_in_col(dense, col, step)
            {
                best = better_pivot(best, candidate);
            }
            largest_fallback = match largest_fallback {
                Some(current) if current.value.abs() >= candidate.value.abs() => Some(current),
                _ => Some(candidate),
            };
        }
    }

    best.or_else(|| {
        largest_fallback.map(|mut candidate| {
            candidate.small = true;
            candidate
        })
    })
}

fn make_candidate(
    dense: &[Vec<f64>],
    row: usize,
    col: usize,
    step: usize,
    rel_threshold: f64,
    abs_threshold: f64,
    method: PivotSelectionMethod,
) -> PivotCandidate {
    let row_count = active_row_count(dense, row, step).saturating_sub(1);
    let col_count = active_col_count(dense, col, step).saturating_sub(1);
    let value = dense[row][col];
    let largest = largest_in_col(dense, col, step);
    PivotCandidate {
        row,
        col,
        value,
        product: row_count * col_count,
        method,
        small: value.abs() <= abs_threshold || value.abs() < rel_threshold * largest,
    }
}

fn candidate_allowed(
    candidate: &PivotCandidate,
    dense: &[Vec<f64>],
    row: usize,
    col: usize,
    step: usize,
    rel_threshold: f64,
    abs_threshold: f64,
) -> bool {
    candidate.value.abs() > ZERO_TOLERANCE
        && candidate.value.abs() > abs_threshold
        && candidate.value.abs() >= rel_threshold * largest_in_col(dense, col, step)
        && active_row_count(dense, row, step) > 0
}

fn better_pivot(
    current: Option<PivotCandidate>,
    candidate: PivotCandidate,
) -> Option<PivotCandidate> {
    match current {
        None => Some(candidate),
        Some(best) => {
            if candidate.product < best.product
                || (candidate.product == best.product && candidate.value.abs() > best.value.abs())
            {
                Some(candidate)
            } else {
                Some(best)
            }
        }
    }
}

fn active_row_count(dense: &[Vec<f64>], row: usize, step: usize) -> usize {
    (step..dense.len())
        .filter(|&col| dense[row][col].abs() > ZERO_TOLERANCE)
        .count()
}

fn active_col_count(dense: &[Vec<f64>], col: usize, step: usize) -> usize {
    (step..dense.len())
        .filter(|&row| dense[row][col].abs() > ZERO_TOLERANCE)
        .count()
}

fn largest_in_col(dense: &[Vec<f64>], col: usize, step: usize) -> f64 {
    (step..dense.len())
        .map(|row| dense[row][col].abs())
        .fold(0.0, f64::max)
}

fn rhs_sparse_excludes(rhs: Option<&[f64]>, one_based_row: usize) -> bool {
    rhs.and_then(|values| values.get(one_based_row - 1))
        .is_some_and(|value| value.abs() <= ZERO_TOLERANCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(left: f64, right: f64) {
        assert!(
            (left - right).abs() < 1.0e-9,
            "expected {left} to be close to {right}"
        );
    }

    fn sample_matrix() -> SparseFactorMatrix {
        let mut matrix = SparseFactorMatrix::new(3).unwrap();
        matrix.set(1, 1, 2.0).unwrap();
        matrix.set(1, 2, 1.0).unwrap();
        matrix.set(2, 1, 4.0).unwrap();
        matrix.set(2, 2, -6.0).unwrap();
        matrix.set(3, 2, 7.0).unwrap();
        matrix.set(3, 3, 2.0).unwrap();
        matrix
    }

    #[test]
    fn order_and_factor_solves_real_sparse_matrix() {
        let mut matrix = sample_matrix();

        let report = matrix
            .order_and_factor(None, PivotOptions::default())
            .unwrap();
        let solution = matrix.solve(&[5.0, -2.0, 9.0]).unwrap();

        assert_eq!(report.pivots.len(), 3);
        assert_close(solution[0], 1.75);
        assert_close(solution[1], 1.5);
        assert_close(solution[2], -0.75);
    }

    #[test]
    fn off_diagonal_pivot_rescues_zero_diagonal() {
        let mut matrix = SparseFactorMatrix::new(2).unwrap();
        matrix.set(1, 2, 3.0).unwrap();
        matrix.set(2, 1, 2.0).unwrap();

        let report = matrix
            .order_and_factor(
                None,
                PivotOptions {
                    diagonal_pivoting: false,
                    ..PivotOptions::default()
                },
            )
            .unwrap();

        assert_eq!(report.pivots[0].method, PivotSelectionMethod::Singleton);
        assert_eq!(report.row_permutation, vec![1, 2]);
        assert_eq!(report.col_permutation, vec![2, 1]);
    }

    #[test]
    fn factor_without_order_reuses_existing_permutation() {
        let mut matrix = sample_matrix();
        let original = matrix
            .order_and_factor(None, PivotOptions::default())
            .unwrap();

        matrix.set(1, 1, 3.0).unwrap();
        matrix.set(1, 2, 1.0).unwrap();
        matrix.set(2, 1, 4.0).unwrap();
        matrix.set(2, 2, -5.0).unwrap();
        matrix.set(3, 2, 7.0).unwrap();
        matrix.set(3, 3, 2.0).unwrap();

        matrix.needs_ordering = false;
        matrix.row_permutation = original.row_permutation;
        matrix.col_permutation = original.col_permutation;
        let refactored = matrix.factor().unwrap();

        assert_eq!(refactored.row_permutation, matrix.row_permutation);
        assert!(matrix.is_factored());
    }

    #[test]
    fn factor_reports_zero_diagonal_when_reorder_is_disabled() {
        let mut matrix = SparseFactorMatrix::new(2).unwrap();
        matrix.set(1, 2, 1.0).unwrap();
        matrix.set(2, 1, 1.0).unwrap();
        matrix.needs_ordering = false;

        assert!(matches!(
            matrix.factor(),
            Err(SparseFactorError::ZeroDiagonal { index: 1 })
        ));
    }

    #[test]
    fn singular_matrix_is_reported_during_ordering() {
        let mut matrix = SparseFactorMatrix::new(3).unwrap();
        matrix.set(1, 1, 1.0).unwrap();
        matrix.set(2, 2, 2.0).unwrap();

        assert!(matches!(
            matrix.order_and_factor(None, PivotOptions::default()),
            Err(SparseFactorError::Singular { .. })
        ));
    }

    #[test]
    fn absolute_threshold_falls_back_to_small_pivot_warning() {
        let mut matrix = SparseFactorMatrix::new(1).unwrap();
        matrix.set(1, 1, 1.0e-6).unwrap();

        let report = matrix
            .order_and_factor(
                None,
                PivotOptions {
                    absolute_threshold: 1.0e-3,
                    ..PivotOptions::default()
                },
            )
            .unwrap();

        assert_eq!(report.warning, Some(FactorWarning::SmallPivot));
        assert_close(report.pivots[0].value, 1.0e-6);
    }

    #[test]
    fn partition_modes_are_applied() {
        let mut matrix = SparseFactorMatrix::new(4).unwrap();
        matrix.set(1, 1, 1.0).unwrap();
        matrix.set(2, 1, 1.0).unwrap();
        matrix.set(3, 1, 1.0).unwrap();
        matrix.set(4, 4, 1.0).unwrap();

        assert_eq!(
            matrix.partition_matrix(PartitionMode::Direct),
            vec![PartitionAddressing::Direct; 4]
        );
        assert_eq!(
            matrix.partition_matrix(PartitionMode::Indirect),
            vec![PartitionAddressing::Indirect; 4]
        );
        assert_eq!(
            matrix.partition_matrix(PartitionMode::Auto)[0],
            PartitionAddressing::Direct
        );
    }

    #[test]
    fn lower_and_upper_reconstruct_permuted_matrix() {
        let mut matrix = sample_matrix();
        matrix
            .order_and_factor(None, PivotOptions::default())
            .unwrap();

        let lower = matrix.lower().unwrap();
        let upper = matrix.upper().unwrap();
        let mut product = vec![vec![0.0; matrix.size()]; matrix.size()];
        for row in 0..matrix.size() {
            for col in 0..matrix.size() {
                product[row][col] = (0..matrix.size())
                    .map(|index| lower[row][index] * upper[index][col])
                    .sum::<f64>();
            }
        }
        let permuted = matrix.to_permuted_dense();

        for row in 0..matrix.size() {
            for col in 0..matrix.size() {
                assert_close(product[row][col], permuted[row][col]);
            }
        }
    }

    #[test]
    fn set_to_zero_removes_structural_entry() {
        let mut matrix = SparseFactorMatrix::new(2).unwrap();
        matrix.set(1, 1, 2.0).unwrap();
        matrix.add(1, 1, -2.0).unwrap();

        assert_close(matrix.get(1, 1).unwrap(), 0.0);
        assert!(matrix.values.is_empty());
    }
}
