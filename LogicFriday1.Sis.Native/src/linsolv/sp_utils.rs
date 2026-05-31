//! Native Rust utilities for `LogicSynthesis/sis/linsolv/spUtils.c`.
//!
//! Sparse1.3 stores one linked element in row and column lists and exposes a
//! collection of optional matrix utilities. This port keeps the behavior in an
//! owned Rust data model: scaling, matrix-vector multiplication, determinant
//! extraction, fill stripping, row/column deletion, norm and growth estimates,
//! roundoff estimation, and a preorder pass for modified nodal matrices.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

const MACHINE_RESOLUTION: f64 = f64::EPSILON;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpComplex {
    pub real: f64,
    pub imag: f64,
}

impl SpComplex {
    pub const ZERO: Self = Self {
        real: 0.0,
        imag: 0.0,
    };
    pub const ONE: Self = Self {
        real: 1.0,
        imag: 0.0,
    };

    pub fn new(real: f64, imag: f64) -> Self {
        Self { real, imag }
    }

    pub fn one_norm(self) -> f64 {
        self.real.abs() + self.imag.abs()
    }

    pub fn is_zero(self) -> bool {
        self.real == 0.0 && self.imag == 0.0
    }
}

impl std::ops::Add for SpComplex {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.real + rhs.real, self.imag + rhs.imag)
    }
}

impl std::ops::AddAssign for SpComplex {
    fn add_assign(&mut self, rhs: Self) {
        self.real += rhs.real;
        self.imag += rhs.imag;
    }
}

impl std::ops::Mul for SpComplex {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(
            self.real * rhs.real - self.imag * rhs.imag,
            self.real * rhs.imag + self.imag * rhs.real,
        )
    }
}

impl std::ops::Mul<f64> for SpComplex {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.real * rhs, self.imag * rhs)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpError {
    IndexOutOfBounds,
    SingularMatrix,
    FactoredMatrixRequired,
    UnfactoredMatrixRequired,
    EmptyScaleVector,
}

impl fmt::Display for SpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IndexOutOfBounds => write!(f, "matrix index is out of bounds"),
            Self::SingularMatrix => write!(f, "matrix is singular"),
            Self::FactoredMatrixRequired => write!(f, "matrix must be factored"),
            Self::UnfactoredMatrixRequired => write!(f, "matrix must not be factored"),
            Self::EmptyScaleVector => write!(f, "scale vectors must cover every external index"),
        }
    }
}

impl Error for SpError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpElementKind {
    Original,
    Fill,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpElement {
    pub row: usize,
    pub col: usize,
    pub value: SpComplex,
    pub kind: SpElementKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpMatrix {
    size: usize,
    elements: BTreeMap<(usize, usize), SpElement>,
    row_external: Vec<usize>,
    col_external: Vec<usize>,
    factored: bool,
    singular: bool,
    reordered: bool,
    odd_interchanges: bool,
    rel_threshold: f64,
    max_row_count_in_lower_tri: Option<usize>,
}

impl SpMatrix {
    pub fn new(size: usize) -> Self {
        let external = (0..=size).collect::<Vec<_>>();
        Self {
            size,
            elements: BTreeMap::new(),
            row_external: external.clone(),
            col_external: external,
            factored: false,
            singular: false,
            reordered: false,
            odd_interchanges: false,
            rel_threshold: 1.0,
            max_row_count_in_lower_tri: None,
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn is_factored(&self) -> bool {
        self.factored
    }

    pub fn set_factored(&mut self, factored: bool) {
        self.factored = factored;
        self.max_row_count_in_lower_tri = None;
    }

    pub fn set_singular(&mut self, singular: bool) {
        self.singular = singular;
    }

    pub fn set_relative_threshold(&mut self, threshold: f64) {
        self.rel_threshold = threshold;
    }

    pub fn is_reordered(&self) -> bool {
        self.reordered
    }

    pub fn odd_interchanges(&self) -> bool {
        self.odd_interchanges
    }

    pub fn set_external_row_order(&mut self, order: Vec<usize>) -> Result<(), SpError> {
        if order.len() != self.size + 1 {
            return Err(SpError::IndexOutOfBounds);
        }
        self.row_external = order;
        Ok(())
    }

    pub fn set_external_col_order(&mut self, order: Vec<usize>) -> Result<(), SpError> {
        if order.len() != self.size + 1 {
            return Err(SpError::IndexOutOfBounds);
        }
        self.col_external = order;
        Ok(())
    }

    pub fn get(&self, row: usize, col: usize) -> Option<SpComplex> {
        self.elements.get(&(row, col)).map(|element| element.value)
    }

    pub fn get_real(&self, row: usize, col: usize) -> Option<f64> {
        self.get(row, col).map(|value| value.real)
    }

    pub fn insert_real(&mut self, row: usize, col: usize, value: f64) -> Result<(), SpError> {
        self.insert(
            row,
            col,
            SpComplex::new(value, 0.0),
            SpElementKind::Original,
        )
    }

    pub fn insert_complex(
        &mut self,
        row: usize,
        col: usize,
        value: SpComplex,
    ) -> Result<(), SpError> {
        self.insert(row, col, value, SpElementKind::Original)
    }

    pub fn insert_fill_real(&mut self, row: usize, col: usize, value: f64) -> Result<(), SpError> {
        self.insert(row, col, SpComplex::new(value, 0.0), SpElementKind::Fill)
    }

    pub fn elements(&self) -> impl Iterator<Item = &SpElement> {
        self.elements.values()
    }

    pub fn scale(
        &mut self,
        rhs_scale_factors: &[f64],
        solution_scale_factors: &[f64],
    ) -> Result<(), SpError> {
        if self.factored {
            return Err(SpError::UnfactoredMatrixRequired);
        }
        self.ensure_scales_cover_external_order(rhs_scale_factors, solution_scale_factors)?;

        for element in self.elements.values_mut() {
            let row_scale = rhs_scale_factors[self.row_external[element.row]];
            let col_scale = solution_scale_factors[self.col_external[element.col]];
            element.value = element.value * row_scale * col_scale;
        }
        Ok(())
    }

    pub fn multiply(&self, solution: &[SpComplex]) -> Result<Vec<SpComplex>, SpError> {
        self.ensure_vector(solution)?;
        let mut rhs = vec![SpComplex::ZERO; self.size + 1];
        for element in self.elements.values() {
            rhs[self.row_external[element.row]] +=
                element.value * solution[self.col_external[element.col]];
        }
        Ok(rhs)
    }

    pub fn multiply_real(&self, solution: &[f64]) -> Result<Vec<f64>, SpError> {
        let solution = solution
            .iter()
            .map(|value| SpComplex::new(*value, 0.0))
            .collect::<Vec<_>>();
        Ok(self
            .multiply(&solution)?
            .into_iter()
            .map(|value| value.real)
            .collect())
    }

    pub fn multiply_transposed(&self, solution: &[SpComplex]) -> Result<Vec<SpComplex>, SpError> {
        self.ensure_vector(solution)?;
        let mut rhs = vec![SpComplex::ZERO; self.size + 1];
        for element in self.elements.values() {
            rhs[self.col_external[element.col]] +=
                element.value * solution[self.row_external[element.row]];
        }
        Ok(rhs)
    }

    pub fn determinant(&self) -> Result<SpDeterminant, SpError> {
        if !self.factored {
            return Err(SpError::FactoredMatrixRequired);
        }
        if self.singular {
            return Ok(SpDeterminant::zero());
        }

        let mut determinant = if self.odd_interchanges {
            SpComplex::new(-1.0, 0.0)
        } else {
            SpComplex::ONE
        };

        for index in 1..=self.size {
            let Some(diag) = self.get(index, index) else {
                return Err(SpError::SingularMatrix);
            };
            determinant = determinant * diag;
        }

        Ok(SpDeterminant::normalized(determinant))
    }

    pub fn strip_fills(&mut self) {
        self.elements
            .retain(|_, element| element.kind == SpElementKind::Original);
    }

    pub fn delete_row_and_col(&mut self, row: usize, col: usize) -> Result<(), SpError> {
        self.check_index(row)?;
        self.check_index(col)?;
        self.elements
            .retain(|(element_row, element_col), _| *element_row != row && *element_col != col);
        self.max_row_count_in_lower_tri = None;
        Ok(())
    }

    pub fn norm(&self) -> Result<f64, SpError> {
        if self.factored {
            return Err(SpError::UnfactoredMatrixRequired);
        }

        let mut sums = vec![0.0; self.size + 1];
        for element in self.elements.values() {
            sums[element.row] += element.value.one_norm();
        }
        Ok(sums.into_iter().fold(0.0, f64::max))
    }

    pub fn largest_element(&self) -> f64 {
        if self.factored {
            if self.singular {
                return 0.0;
            }

            let mut max_row = 0.0;
            let mut max_col = 0.0;
            for index in 1..=self.size {
                if let Some(diag) = self.get(index, index) {
                    let pivot = reciprocal_norm(diag);
                    max_row = f64::max(max_row, pivot);
                }

                for element in self
                    .row_elements(index)
                    .filter(|element| element.col < index)
                {
                    max_row = f64::max(max_row, element.value.one_norm());
                }

                let mut abs_col_sum = 1.0;
                for element in self
                    .col_elements(index)
                    .filter(|element| element.row < index)
                {
                    abs_col_sum += element.value.one_norm();
                }
                max_col = f64::max(max_col, abs_col_sum);
            }

            return max_row * max_col;
        }

        self.elements
            .values()
            .map(|element| element.value.one_norm())
            .fold(0.0, f64::max)
    }

    pub fn pseudo_condition(&self) -> Result<f64, SpError> {
        if !self.factored {
            return Err(SpError::FactoredMatrixRequired);
        }
        if self.singular {
            return Ok(0.0);
        }

        let mut max = 0.0;
        let mut min = f64::INFINITY;
        for index in 1..=self.size {
            let Some(diag) = self.get(index, index) else {
                return Ok(0.0);
            };
            let magnitude = diag.one_norm();
            max = f64::max(max, magnitude);
            min = f64::min(min, magnitude);
        }

        if max == 0.0 || min == 0.0 {
            Ok(0.0)
        } else {
            Ok(min / max)
        }
    }

    pub fn condition(&self, norm_of_matrix: f64) -> Result<f64, SpError> {
        if !self.factored {
            return Err(SpError::FactoredMatrixRequired);
        }
        if self.singular || norm_of_matrix == 0.0 {
            return Ok(0.0);
        }

        let inverse_norm = self.inverse_one_norm_estimate()?;
        if inverse_norm == 0.0 {
            Ok(0.0)
        } else {
            Ok(1.0 / (norm_of_matrix * inverse_norm))
        }
    }

    pub fn roundoff(&mut self, rho: Option<f64>) -> Result<f64, SpError> {
        if !self.factored {
            return Err(SpError::FactoredMatrixRequired);
        }

        let rho = rho.unwrap_or_else(|| self.largest_element());
        let max_count = match self.max_row_count_in_lower_tri {
            Some(count) => count,
            None => {
                let count = (1..=self.size)
                    .map(|row| {
                        self.row_elements(row)
                            .filter(|element| element.col < row)
                            .count()
                    })
                    .max()
                    .unwrap_or(0);
                self.max_row_count_in_lower_tri = Some(count);
                count
            }
        };

        let max_count = max_count as f64;
        let gear = 1.01 * ((max_count + 1.0) * self.rel_threshold + 1.0) * max_count.powi(2);
        let reid = 3.01 * self.size as f64;
        Ok(MACHINE_RESOLUTION * rho * f64::min(gear, reid))
    }

    pub fn mna_preorder(&mut self) -> Result<(), SpError> {
        if self.factored {
            return Err(SpError::UnfactoredMatrixRequired);
        }

        self.reordered = true;
        let mut start_at = 1;
        loop {
            let mut another_pass_needed = false;
            let mut swapped = false;

            for col in start_at..=self.size {
                if self.get(col, col).is_none() {
                    let twins = self.count_twins(col);
                    if twins.len() == 1 {
                        self.swap_cols(col, twins[0]);
                        swapped = true;
                    } else if twins.len() > 1 && !another_pass_needed {
                        another_pass_needed = true;
                        start_at = col;
                    }
                }
            }

            if another_pass_needed {
                for col in start_at..=self.size {
                    if swapped {
                        break;
                    }
                    if self.get(col, col).is_none() {
                        let twins = self.count_twins(col);
                        if let Some(twin_row) = twins.first() {
                            self.swap_cols(col, *twin_row);
                            swapped = true;
                        }
                    }
                }
            }

            if !another_pass_needed {
                break;
            }
        }

        Ok(())
    }

    fn insert(
        &mut self,
        row: usize,
        col: usize,
        value: SpComplex,
        kind: SpElementKind,
    ) -> Result<(), SpError> {
        self.check_index(row)?;
        self.check_index(col)?;
        if value.is_zero() {
            self.elements.remove(&(row, col));
        } else {
            self.elements.insert(
                (row, col),
                SpElement {
                    row,
                    col,
                    value,
                    kind,
                },
            );
        }
        Ok(())
    }

    fn check_index(&self, index: usize) -> Result<(), SpError> {
        if index == 0 || index > self.size {
            Err(SpError::IndexOutOfBounds)
        } else {
            Ok(())
        }
    }

    fn ensure_vector(&self, vector: &[SpComplex]) -> Result<(), SpError> {
        if vector.len() <= self.size {
            Err(SpError::IndexOutOfBounds)
        } else {
            Ok(())
        }
    }

    fn ensure_scales_cover_external_order(
        &self,
        rhs_scale_factors: &[f64],
        solution_scale_factors: &[f64],
    ) -> Result<(), SpError> {
        let max_row = self.row_external.iter().copied().max().unwrap_or(0);
        let max_col = self.col_external.iter().copied().max().unwrap_or(0);
        if rhs_scale_factors.len() <= max_row || solution_scale_factors.len() <= max_col {
            Err(SpError::EmptyScaleVector)
        } else {
            Ok(())
        }
    }

    fn row_elements(&self, row: usize) -> impl Iterator<Item = &SpElement> {
        self.elements
            .values()
            .filter(move |element| element.row == row)
    }

    fn col_elements(&self, col: usize) -> impl Iterator<Item = &SpElement> {
        self.elements
            .values()
            .filter(move |element| element.col == col)
    }

    fn count_twins(&self, col: usize) -> Vec<usize> {
        let mut twins = Vec::new();
        for element in self.col_elements(col) {
            if element.value.real.abs() == 1.0 {
                let row = element.row;
                if self
                    .get(col, row)
                    .is_some_and(|value| value.real.abs() == 1.0)
                {
                    twins.push(row);
                    if twins.len() >= 2 {
                        break;
                    }
                }
            }
        }
        twins
    }

    fn swap_cols(&mut self, col1: usize, col2: usize) {
        if col1 == col2 {
            return;
        }

        let moved = self
            .elements
            .values()
            .copied()
            .map(|mut element| {
                if element.col == col1 {
                    element.col = col2;
                } else if element.col == col2 {
                    element.col = col1;
                }
                ((element.row, element.col), element)
            })
            .collect::<BTreeMap<_, _>>();
        self.elements = moved;
        self.col_external.swap(col1, col2);
        self.odd_interchanges = !self.odd_interchanges;
    }

    fn inverse_one_norm_estimate(&self) -> Result<f64, SpError> {
        let inverse = self.inverse_dense()?;
        let mut max_col_sum = 0.0;
        for col in 0..self.size {
            let sum = inverse.iter().map(|row| row[col].one_norm()).sum::<f64>();
            max_col_sum = f64::max(max_col_sum, sum);
        }
        Ok(max_col_sum)
    }

    fn inverse_dense(&self) -> Result<Vec<Vec<SpComplex>>, SpError> {
        let n = self.size;
        let mut augmented = vec![vec![SpComplex::ZERO; n * 2]; n];
        for row in 0..n {
            for col in 0..n {
                augmented[row][col] = self.get(row + 1, col + 1).unwrap_or(SpComplex::ZERO);
            }
            augmented[row][n + row] = SpComplex::ONE;
        }

        for pivot_col in 0..n {
            let pivot_row = (pivot_col..n)
                .max_by(|left, right| {
                    augmented[*left][pivot_col]
                        .one_norm()
                        .total_cmp(&augmented[*right][pivot_col].one_norm())
                })
                .expect("pivot range is not empty");
            if augmented[pivot_row][pivot_col].one_norm() == 0.0 {
                return Err(SpError::SingularMatrix);
            }
            augmented.swap(pivot_col, pivot_row);

            let pivot = augmented[pivot_col][pivot_col];
            for col in 0..(n * 2) {
                augmented[pivot_col][col] = divide(augmented[pivot_col][col], pivot);
            }

            for row in 0..n {
                if row == pivot_col {
                    continue;
                }

                let factor = augmented[row][pivot_col];
                if factor.is_zero() {
                    continue;
                }

                for col in 0..(n * 2) {
                    let value = augmented[row][col] + (augmented[pivot_col][col] * factor * -1.0);
                    augmented[row][col] = value;
                }
            }
        }

        Ok(augmented.into_iter().map(|row| row[n..].to_vec()).collect())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpDeterminant {
    pub exponent: i32,
    pub determinant: SpComplex,
}

impl SpDeterminant {
    fn zero() -> Self {
        Self {
            exponent: 0,
            determinant: SpComplex::ZERO,
        }
    }

    fn normalized(mut determinant: SpComplex) -> Self {
        if determinant.is_zero() {
            return Self::zero();
        }

        let mut exponent = 0;
        while determinant.one_norm() >= 10.0 {
            determinant = determinant * 0.1;
            exponent += 1;
        }
        while determinant.one_norm() < 1.0 {
            determinant = determinant * 10.0;
            exponent -= 1;
        }

        Self {
            exponent,
            determinant,
        }
    }
}

fn reciprocal_norm(value: SpComplex) -> f64 {
    let denominator = value.real * value.real + value.imag * value.imag;
    if denominator == 0.0 {
        0.0
    } else {
        SpComplex::new(value.real / denominator, -value.imag / denominator).one_norm()
    }
}

fn divide(left: SpComplex, right: SpComplex) -> SpComplex {
    let denominator = right.real * right.real + right.imag * right.imag;
    SpComplex::new(
        (left.real * right.real + left.imag * right.imag) / denominator,
        (left.imag * right.real - left.real * right.imag) / denominator,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_rows_and_columns_with_external_order() {
        let mut matrix = SpMatrix::new(2);
        matrix.insert_real(1, 1, 2.0).unwrap();
        matrix.insert_real(1, 2, 3.0).unwrap();
        matrix.insert_real(2, 1, 5.0).unwrap();
        matrix.set_external_row_order(vec![0, 2, 1]).unwrap();
        matrix.set_external_col_order(vec![0, 1, 2]).unwrap();

        matrix.scale(&[1.0, 7.0, 11.0], &[1.0, 13.0, 17.0]).unwrap();

        assert_eq!(matrix.get_real(1, 1), Some(2.0 * 11.0 * 13.0));
        assert_eq!(matrix.get_real(1, 2), Some(3.0 * 11.0 * 17.0));
        assert_eq!(matrix.get_real(2, 1), Some(5.0 * 7.0 * 13.0));
    }

    #[test]
    fn multiplies_matrix_and_transpose_using_external_order() {
        let mut matrix = SpMatrix::new(2);
        matrix.insert_real(1, 1, 2.0).unwrap();
        matrix.insert_real(1, 2, 3.0).unwrap();
        matrix.insert_real(2, 1, 5.0).unwrap();

        assert_eq!(
            matrix.multiply_real(&[0.0, 7.0, 11.0]).unwrap(),
            vec![0.0, 47.0, 35.0]
        );
        let transposed = matrix
            .multiply_transposed(&[
                SpComplex::ZERO,
                SpComplex::new(7.0, 0.0),
                SpComplex::new(11.0, 0.0),
            ])
            .unwrap();

        assert_eq!(transposed[1].real, 69.0);
        assert_eq!(transposed[2].real, 21.0);
    }

    #[test]
    fn handles_complex_multiply() {
        let mut matrix = SpMatrix::new(1);
        matrix
            .insert_complex(1, 1, SpComplex::new(2.0, 3.0))
            .unwrap();

        let rhs = matrix
            .multiply(&[SpComplex::ZERO, SpComplex::new(5.0, -1.0)])
            .unwrap();

        assert_eq!(rhs[1], SpComplex::new(13.0, 13.0));
    }

    #[test]
    fn normalizes_determinant_and_tracks_interchanges() {
        let mut matrix = SpMatrix::new(2);
        matrix.insert_real(1, 2, 1.0).unwrap();
        matrix.insert_real(2, 1, 1.0).unwrap();
        matrix.mna_preorder().unwrap();
        matrix.set_factored(true);

        let determinant = matrix.determinant().unwrap();

        assert_eq!(determinant.exponent, 0);
        assert_eq!(determinant.determinant.real, -1.0);
    }

    #[test]
    fn strips_fills_and_deletes_row_and_column() {
        let mut matrix = SpMatrix::new(3);
        matrix.insert_real(1, 1, 1.0).unwrap();
        matrix.insert_fill_real(1, 2, 2.0).unwrap();
        matrix.insert_real(2, 3, 3.0).unwrap();
        matrix.insert_real(3, 2, 4.0).unwrap();

        matrix.strip_fills();
        assert_eq!(matrix.get_real(1, 2), None);

        matrix.delete_row_and_col(2, 2).unwrap();
        assert_eq!(matrix.get_real(2, 3), None);
        assert_eq!(matrix.get_real(3, 2), None);
        assert_eq!(matrix.get_real(1, 1), Some(1.0));
    }

    #[test]
    fn computes_norm_and_largest_unfactored_element() {
        let mut matrix = SpMatrix::new(2);
        matrix.insert_real(1, 1, -2.0).unwrap();
        matrix
            .insert_complex(1, 2, SpComplex::new(3.0, -4.0))
            .unwrap();
        matrix.insert_real(2, 1, 5.0).unwrap();

        assert_eq!(matrix.norm().unwrap(), 9.0);
        assert_eq!(matrix.largest_element(), 7.0);
    }

    #[test]
    fn computes_pseudo_condition_and_condition() {
        let mut matrix = SpMatrix::new(2);
        matrix.insert_real(1, 1, 2.0).unwrap();
        matrix.insert_real(2, 2, 4.0).unwrap();
        let norm = matrix.norm().unwrap();
        matrix.set_factored(true);

        assert_eq!(matrix.pseudo_condition().unwrap(), 0.5);
        assert!((matrix.condition(norm).unwrap() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn estimates_factored_largest_element_and_roundoff() {
        let mut matrix = SpMatrix::new(3);
        matrix.insert_real(1, 1, 2.0).unwrap();
        matrix.insert_real(2, 1, 6.0).unwrap();
        matrix.insert_real(2, 2, 3.0).unwrap();
        matrix.insert_real(3, 1, 5.0).unwrap();
        matrix.insert_real(3, 2, 7.0).unwrap();
        matrix.insert_real(3, 3, 4.0).unwrap();
        matrix.set_factored(true);
        matrix.set_relative_threshold(0.25);

        assert_eq!(matrix.largest_element(), 7.0);
        let roundoff = matrix.roundoff(Some(14.0)).unwrap();

        assert!(roundoff > 0.0);
        assert!(roundoff < 1e-12);
    }

    #[test]
    fn preorders_lone_twins_by_swapping_columns() {
        let mut matrix = SpMatrix::new(3);
        matrix.insert_real(1, 3, 1.0).unwrap();
        matrix.insert_real(3, 1, 1.0).unwrap();
        matrix.insert_real(2, 2, 2.0).unwrap();

        matrix.mna_preorder().unwrap();

        assert!(matrix.is_reordered());
        assert!(matrix.odd_interchanges());
        assert_eq!(matrix.get_real(1, 1), Some(1.0));
        assert_eq!(matrix.get_real(3, 3), Some(1.0));
    }

    #[test]
    fn reports_invalid_operations() {
        let mut matrix = SpMatrix::new(1);
        assert_eq!(
            matrix.insert_real(0, 1, 1.0),
            Err(SpError::IndexOutOfBounds)
        );
        assert_eq!(matrix.scale(&[], &[]), Err(SpError::EmptyScaleVector));
        matrix.insert_real(1, 1, 1.0).unwrap();
        matrix.set_factored(true);

        assert_eq!(matrix.norm(), Err(SpError::UnfactoredMatrixRequired));
        assert_eq!(
            matrix.mna_preorder(),
            Err(SpError::UnfactoredMatrixRequired)
        );
    }
}
