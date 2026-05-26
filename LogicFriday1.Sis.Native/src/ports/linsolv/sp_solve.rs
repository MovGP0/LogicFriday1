use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ComplexNumber {
    pub real: f64,
    pub imag: f64,
}

impl ComplexNumber {
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

    pub fn is_zero(self) -> bool {
        self.real == 0.0 && self.imag == 0.0
    }
}

impl std::ops::Add for ComplexNumber {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.real + rhs.real, self.imag + rhs.imag)
    }
}

impl std::ops::AddAssign for ComplexNumber {
    fn add_assign(&mut self, rhs: Self) {
        self.real += rhs.real;
        self.imag += rhs.imag;
    }
}

impl std::ops::Sub for ComplexNumber {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.real - rhs.real, self.imag - rhs.imag)
    }
}

impl std::ops::SubAssign for ComplexNumber {
    fn sub_assign(&mut self, rhs: Self) {
        self.real -= rhs.real;
        self.imag -= rhs.imag;
    }
}

impl std::ops::Mul for ComplexNumber {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(
            self.real * rhs.real - self.imag * rhs.imag,
            self.real * rhs.imag + self.imag * rhs.real,
        )
    }
}

impl std::ops::Mul<f64> for ComplexNumber {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.real * rhs, self.imag * rhs)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SparseSolveEntry {
    pub index: usize,
    pub value: ComplexNumber,
}

impl SparseSolveEntry {
    pub fn real(index: usize, value: f64) -> Self {
        Self {
            index,
            value: ComplexNumber::new(value, 0.0),
        }
    }

    pub fn complex(index: usize, real: f64, imag: f64) -> Self {
        Self {
            index,
            value: ComplexNumber::new(real, imag),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SparseSolvePivot {
    pub inverse: ComplexNumber,
    pub lower_column: Vec<SparseSolveEntry>,
    pub upper_row: Vec<SparseSolveEntry>,
}

impl SparseSolvePivot {
    pub fn real(inverse: f64) -> Self {
        Self {
            inverse: ComplexNumber::new(inverse, 0.0),
            lower_column: Vec::new(),
            upper_row: Vec::new(),
        }
    }

    pub fn complex(inverse: ComplexNumber) -> Self {
        Self {
            inverse,
            lower_column: Vec::new(),
            upper_row: Vec::new(),
        }
    }

    pub fn with_lower(mut self, entries: impl IntoIterator<Item = SparseSolveEntry>) -> Self {
        self.lower_column.extend(entries);
        self
    }

    pub fn with_upper(mut self, entries: impl IntoIterator<Item = SparseSolveEntry>) -> Self {
        self.upper_row.extend(entries);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FactorizedSparseMatrix {
    pivots: Vec<SparseSolvePivot>,
    int_to_ext_row: Vec<usize>,
    int_to_ext_col: Vec<usize>,
}

impl FactorizedSparseMatrix {
    pub fn identity(size: usize) -> Self {
        Self {
            pivots: vec![SparseSolvePivot::real(1.0); size],
            int_to_ext_row: (0..size).collect(),
            int_to_ext_col: (0..size).collect(),
        }
    }

    pub fn with_maps(
        pivots: Vec<SparseSolvePivot>,
        int_to_ext_row: Vec<usize>,
        int_to_ext_col: Vec<usize>,
    ) -> Result<Self, SparseSolveError> {
        let size = pivots.len();

        if int_to_ext_row.len() != size {
            return Err(SparseSolveError::InvalidMapLength {
                name: "row",
                expected: size,
                actual: int_to_ext_row.len(),
            });
        }

        if int_to_ext_col.len() != size {
            return Err(SparseSolveError::InvalidMapLength {
                name: "column",
                expected: size,
                actual: int_to_ext_col.len(),
            });
        }

        let matrix = Self {
            pivots,
            int_to_ext_row,
            int_to_ext_col,
        };
        matrix.validate()?;
        Ok(matrix)
    }

    pub fn from_real_lu(
        diagonal_inverse: impl IntoIterator<Item = f64>,
        lower_columns: Vec<Vec<(usize, f64)>>,
        upper_rows: Vec<Vec<(usize, f64)>>,
    ) -> Result<Self, SparseSolveError> {
        let mut pivots: Vec<SparseSolvePivot> = diagonal_inverse
            .into_iter()
            .map(SparseSolvePivot::real)
            .collect();

        if lower_columns.len() != pivots.len() {
            return Err(SparseSolveError::InvalidPartCount {
                name: "lower",
                expected: pivots.len(),
                actual: lower_columns.len(),
            });
        }

        if upper_rows.len() != pivots.len() {
            return Err(SparseSolveError::InvalidPartCount {
                name: "upper",
                expected: pivots.len(),
                actual: upper_rows.len(),
            });
        }

        for (pivot, entries) in pivots.iter_mut().zip(lower_columns) {
            pivot.lower_column = entries
                .into_iter()
                .map(|(index, value)| SparseSolveEntry::real(index, value))
                .collect();
        }

        for (pivot, entries) in pivots.iter_mut().zip(upper_rows) {
            pivot.upper_row = entries
                .into_iter()
                .map(|(index, value)| SparseSolveEntry::real(index, value))
                .collect();
        }

        let size = pivots.len();
        Self::with_maps(pivots, (0..size).collect(), (0..size).collect())
    }

    pub fn size(&self) -> usize {
        self.pivots.len()
    }

    pub fn solve_real(&self, rhs: &[f64]) -> Result<Vec<f64>, SparseSolveError> {
        self.validate_rhs(rhs.len(), &self.int_to_ext_row, "RHS")?;

        let mut intermediate = vec![0.0; self.size()];
        for internal in (0..self.size()).rev() {
            intermediate[internal] = rhs[self.int_to_ext_row[internal]];
        }

        for internal in 0..self.size() {
            let mut temp = intermediate[internal];
            if temp != 0.0 {
                let pivot = &self.pivots[internal];
                if pivot.inverse.imag != 0.0 {
                    return Err(SparseSolveError::ComplexMatrixUsedForRealSolve);
                }

                temp *= pivot.inverse.real;
                intermediate[internal] = temp;

                for entry in &pivot.lower_column {
                    if entry.value.imag != 0.0 {
                        return Err(SparseSolveError::ComplexMatrixUsedForRealSolve);
                    }

                    intermediate[entry.index] -= temp * entry.value.real;
                }
            }
        }

        for internal in (0..self.size()).rev() {
            let pivot = &self.pivots[internal];
            let mut temp = intermediate[internal];

            for entry in &pivot.upper_row {
                if entry.value.imag != 0.0 {
                    return Err(SparseSolveError::ComplexMatrixUsedForRealSolve);
                }

                temp -= entry.value.real * intermediate[entry.index];
            }

            intermediate[internal] = temp;
        }

        let mut solution = vec![0.0; required_len(&self.int_to_ext_col)];
        for internal in (0..self.size()).rev() {
            solution[self.int_to_ext_col[internal]] = intermediate[internal];
        }

        Ok(solution)
    }

    pub fn solve_transposed_real(&self, rhs: &[f64]) -> Result<Vec<f64>, SparseSolveError> {
        self.validate_rhs(rhs.len(), &self.int_to_ext_col, "RHS")?;

        let mut intermediate = vec![0.0; self.size()];
        for internal in (0..self.size()).rev() {
            intermediate[internal] = rhs[self.int_to_ext_col[internal]];
        }

        for internal in 0..self.size() {
            let temp = intermediate[internal];
            if temp != 0.0 {
                let pivot = &self.pivots[internal];
                for entry in &pivot.upper_row {
                    if entry.value.imag != 0.0 {
                        return Err(SparseSolveError::ComplexMatrixUsedForRealSolve);
                    }

                    intermediate[entry.index] -= temp * entry.value.real;
                }
            }
        }

        for internal in (0..self.size()).rev() {
            let pivot = &self.pivots[internal];
            if pivot.inverse.imag != 0.0 {
                return Err(SparseSolveError::ComplexMatrixUsedForRealSolve);
            }

            let mut temp = intermediate[internal];
            for entry in &pivot.lower_column {
                if entry.value.imag != 0.0 {
                    return Err(SparseSolveError::ComplexMatrixUsedForRealSolve);
                }

                temp -= entry.value.real * intermediate[entry.index];
            }

            intermediate[internal] = temp * pivot.inverse.real;
        }

        let mut solution = vec![0.0; required_len(&self.int_to_ext_row)];
        for internal in (0..self.size()).rev() {
            solution[self.int_to_ext_row[internal]] = intermediate[internal];
        }

        Ok(solution)
    }

    pub fn solve_complex(
        &self,
        rhs: &[ComplexNumber],
    ) -> Result<Vec<ComplexNumber>, SparseSolveError> {
        self.validate_rhs(rhs.len(), &self.int_to_ext_row, "RHS")?;

        let mut intermediate = vec![ComplexNumber::ZERO; self.size()];
        for internal in (0..self.size()).rev() {
            intermediate[internal] = rhs[self.int_to_ext_row[internal]];
        }

        for internal in 0..self.size() {
            let mut temp = intermediate[internal];
            if !temp.is_zero() {
                let pivot = &self.pivots[internal];
                temp = temp * pivot.inverse;
                intermediate[internal] = temp;

                for entry in &pivot.lower_column {
                    intermediate[entry.index] -= temp * entry.value;
                }
            }
        }

        for internal in (0..self.size()).rev() {
            let pivot = &self.pivots[internal];
            let mut temp = intermediate[internal];

            for entry in &pivot.upper_row {
                temp -= entry.value * intermediate[entry.index];
            }

            intermediate[internal] = temp;
        }

        let mut solution = vec![ComplexNumber::ZERO; required_len(&self.int_to_ext_col)];
        for internal in (0..self.size()).rev() {
            solution[self.int_to_ext_col[internal]] = intermediate[internal];
        }

        Ok(solution)
    }

    pub fn solve_transposed_complex(
        &self,
        rhs: &[ComplexNumber],
    ) -> Result<Vec<ComplexNumber>, SparseSolveError> {
        self.validate_rhs(rhs.len(), &self.int_to_ext_col, "RHS")?;

        let mut intermediate = vec![ComplexNumber::ZERO; self.size()];
        for internal in (0..self.size()).rev() {
            intermediate[internal] = rhs[self.int_to_ext_col[internal]];
        }

        for internal in 0..self.size() {
            let temp = intermediate[internal];
            if !temp.is_zero() {
                let pivot = &self.pivots[internal];
                for entry in &pivot.upper_row {
                    intermediate[entry.index] -= temp * entry.value;
                }
            }
        }

        for internal in (0..self.size()).rev() {
            let pivot = &self.pivots[internal];
            let mut temp = intermediate[internal];

            for entry in &pivot.lower_column {
                temp -= intermediate[entry.index] * entry.value;
            }

            intermediate[internal] = temp * pivot.inverse;
        }

        let mut solution = vec![ComplexNumber::ZERO; required_len(&self.int_to_ext_row)];
        for internal in (0..self.size()).rev() {
            solution[self.int_to_ext_row[internal]] = intermediate[internal];
        }

        Ok(solution)
    }

    pub fn solve_complex_separated(
        &self,
        real_rhs: &[f64],
        imag_rhs: &[f64],
    ) -> Result<(Vec<f64>, Vec<f64>), SparseSolveError> {
        if real_rhs.len() != imag_rhs.len() {
            return Err(SparseSolveError::MismatchedComplexVectorLength {
                real: real_rhs.len(),
                imag: imag_rhs.len(),
            });
        }

        let rhs = real_rhs
            .iter()
            .zip(imag_rhs)
            .map(|(real, imag)| ComplexNumber::new(*real, *imag))
            .collect::<Vec<_>>();
        let solution = self.solve_complex(&rhs)?;
        Ok(split_complex_vector(&solution))
    }

    fn validate(&self) -> Result<(), SparseSolveError> {
        for (pivot_index, pivot) in self.pivots.iter().enumerate() {
            for entry in &pivot.lower_column {
                if entry.index >= self.size() {
                    return Err(SparseSolveError::InternalIndexOutOfBounds {
                        index: entry.index,
                        size: self.size(),
                    });
                }

                if entry.index <= pivot_index {
                    return Err(SparseSolveError::InvalidLowerEntry {
                        pivot: pivot_index,
                        row: entry.index,
                    });
                }
            }

            for entry in &pivot.upper_row {
                if entry.index >= self.size() {
                    return Err(SparseSolveError::InternalIndexOutOfBounds {
                        index: entry.index,
                        size: self.size(),
                    });
                }

                if entry.index <= pivot_index {
                    return Err(SparseSolveError::InvalidUpperEntry {
                        pivot: pivot_index,
                        column: entry.index,
                    });
                }
            }
        }

        Ok(())
    }

    fn validate_rhs(
        &self,
        len: usize,
        external_map: &[usize],
        name: &'static str,
    ) -> Result<(), SparseSolveError> {
        let required = required_len(external_map);
        if len < required {
            return Err(SparseSolveError::VectorTooShort {
                name,
                required,
                actual: len,
            });
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SparseSolveError {
    InvalidMapLength {
        name: &'static str,
        expected: usize,
        actual: usize,
    },
    InvalidPartCount {
        name: &'static str,
        expected: usize,
        actual: usize,
    },
    InternalIndexOutOfBounds {
        index: usize,
        size: usize,
    },
    InvalidLowerEntry {
        pivot: usize,
        row: usize,
    },
    InvalidUpperEntry {
        pivot: usize,
        column: usize,
    },
    VectorTooShort {
        name: &'static str,
        required: usize,
        actual: usize,
    },
    ComplexMatrixUsedForRealSolve,
    MismatchedComplexVectorLength {
        real: usize,
        imag: usize,
    },
}

impl fmt::Display for SparseSolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SparseSolveError::InvalidMapLength {
                name,
                expected,
                actual,
            } => write!(
                formatter,
                "invalid {name} map length: expected {expected}, got {actual}"
            ),
            SparseSolveError::InvalidPartCount {
                name,
                expected,
                actual,
            } => write!(
                formatter,
                "invalid {name} part count: expected {expected}, got {actual}"
            ),
            SparseSolveError::InternalIndexOutOfBounds { index, size } => write!(
                formatter,
                "internal index {index} is outside matrix size {size}"
            ),
            SparseSolveError::InvalidLowerEntry { pivot, row } => {
                write!(
                    formatter,
                    "lower entry row {row} is not below pivot {pivot}"
                )
            }
            SparseSolveError::InvalidUpperEntry { pivot, column } => write!(
                formatter,
                "upper entry column {column} is not right of pivot {pivot}"
            ),
            SparseSolveError::VectorTooShort {
                name,
                required,
                actual,
            } => write!(
                formatter,
                "{name} vector is too short: required at least {required}, got {actual}"
            ),
            SparseSolveError::ComplexMatrixUsedForRealSolve => {
                write!(
                    formatter,
                    "complex matrix cannot be solved as a real matrix"
                )
            }
            SparseSolveError::MismatchedComplexVectorLength { real, imag } => write!(
                formatter,
                "complex vector parts differ in length: real {real}, imaginary {imag}"
            ),
        }
    }
}

impl Error for SparseSolveError {}

pub fn split_complex_vector(values: &[ComplexNumber]) -> (Vec<f64>, Vec<f64>) {
    let mut real = Vec::with_capacity(values.len());
    let mut imag = Vec::with_capacity(values.len());

    for value in values {
        real.push(value.real);
        imag.push(value.imag);
    }

    (real, imag)
}

fn required_len(map: &[usize]) -> usize {
    map.iter().copied().max().map_or(0, |index| index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1.0e-9;

    fn assert_close(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual - expected).abs() < EPSILON,
                "expected {expected}, got {actual}"
            );
        }
    }

    fn assert_complex_close(actual: &[ComplexNumber], expected: &[ComplexNumber]) {
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual.real - expected.real).abs() < EPSILON,
                "expected real {}, got {}",
                expected.real,
                actual.real
            );
            assert!(
                (actual.imag - expected.imag).abs() < EPSILON,
                "expected imag {}, got {}",
                expected.imag,
                actual.imag
            );
        }
    }

    fn sample_real_matrix() -> FactorizedSparseMatrix {
        FactorizedSparseMatrix::from_real_lu(
            [0.5, 1.0 / 3.0],
            vec![vec![(1, 1.0)], vec![]],
            vec![vec![(1, 0.5)], vec![]],
        )
        .unwrap()
    }

    #[test]
    fn solves_real_factorized_matrix() {
        let matrix = sample_real_matrix();

        let solution = matrix.solve_real(&[5.0, 5.0]).unwrap();

        assert_close(&solution, &[25.0 / 12.0, 5.0 / 6.0]);
    }

    #[test]
    fn real_solve_preserves_rhs() {
        let matrix = sample_real_matrix();
        let rhs = vec![5.0, 5.0];

        let _ = matrix.solve_real(&rhs).unwrap();

        assert_eq!(rhs, vec![5.0, 5.0]);
    }

    #[test]
    fn solves_real_matrix_in_place_compatible_order() {
        let matrix = sample_real_matrix();
        let mut vector = vec![5.0, 5.0];

        vector = matrix.solve_real(&vector).unwrap();

        assert_close(&vector, &[25.0 / 12.0, 5.0 / 6.0]);
    }

    #[test]
    fn applies_external_row_and_column_maps() {
        let pivots = vec![
            SparseSolvePivot::real(0.5)
                .with_lower([SparseSolveEntry::real(1, 1.0)])
                .with_upper([SparseSolveEntry::real(1, 0.5)]),
            SparseSolvePivot::real(1.0 / 3.0),
        ];
        let matrix = FactorizedSparseMatrix::with_maps(pivots, vec![2, 0], vec![1, 3]).unwrap();

        let solution = matrix.solve_real(&[5.0, 99.0, 5.0]).unwrap();

        assert_close(&solution, &[0.0, 25.0 / 12.0, 0.0, 5.0 / 6.0]);
    }

    #[test]
    fn skips_sparse_real_elimination_when_intermediate_is_zero() {
        let matrix = FactorizedSparseMatrix::from_real_lu(
            [0.5, 1.0 / 3.0],
            vec![vec![(1, 9.0)], vec![]],
            vec![vec![(1, 4.0)], vec![]],
        )
        .unwrap();

        let solution = matrix.solve_real(&[0.0, 6.0]).unwrap();

        assert_close(&solution, &[-8.0, 2.0]);
    }

    #[test]
    fn solves_transposed_real_factorized_matrix() {
        let matrix = sample_real_matrix();

        let solution = matrix.solve_transposed_real(&[6.0, 3.0]).unwrap();

        assert_close(&solution, &[3.0, 0.0]);
    }

    #[test]
    fn solves_complex_factorized_matrix() {
        let pivots = vec![
            SparseSolvePivot::complex(ComplexNumber::new(0.5, 0.0))
                .with_lower([SparseSolveEntry::complex(1, 1.0, 1.0)])
                .with_upper([SparseSolveEntry::complex(1, 0.0, 1.0)]),
            SparseSolvePivot::complex(ComplexNumber::new(1.0 / 3.0, 0.0)),
        ];
        let matrix = FactorizedSparseMatrix::with_maps(pivots, vec![0, 1], vec![0, 1]).unwrap();

        let solution = matrix
            .solve_complex(&[ComplexNumber::new(3.0, 1.0), ComplexNumber::new(5.0, 6.0)])
            .unwrap();

        assert_complex_close(
            &solution,
            &[
                ComplexNumber::new(17.0 / 6.0, -5.0 / 6.0),
                ComplexNumber::new(4.0 / 3.0, 4.0 / 3.0),
            ],
        );
    }

    #[test]
    fn solves_transposed_complex_factorized_matrix() {
        let pivots = vec![
            SparseSolvePivot::complex(ComplexNumber::new(0.5, 0.0))
                .with_lower([SparseSolveEntry::complex(1, 1.0, 1.0)])
                .with_upper([SparseSolveEntry::complex(1, 0.0, 1.0)]),
            SparseSolvePivot::complex(ComplexNumber::new(1.0 / 3.0, 0.0)),
        ];
        let matrix = FactorizedSparseMatrix::with_maps(pivots, vec![0, 1], vec![0, 1]).unwrap();

        let solution = matrix
            .solve_transposed_complex(&[ComplexNumber::new(3.0, 1.0), ComplexNumber::new(5.0, 4.0)])
            .unwrap();

        assert_complex_close(
            &solution,
            &[
                ComplexNumber::new(2.0 / 3.0, -2.0 / 3.0),
                ComplexNumber::new(2.0, 1.0 / 3.0),
            ],
        );
    }

    #[test]
    fn solves_separated_complex_vectors() {
        let pivots = vec![
            SparseSolvePivot::complex(ComplexNumber::new(0.5, 0.0))
                .with_lower([SparseSolveEntry::complex(1, 1.0, 1.0)])
                .with_upper([SparseSolveEntry::complex(1, 0.0, 1.0)]),
            SparseSolvePivot::complex(ComplexNumber::new(1.0 / 3.0, 0.0)),
        ];
        let matrix = FactorizedSparseMatrix::with_maps(pivots, vec![0, 1], vec![0, 1]).unwrap();

        let (real, imag) = matrix
            .solve_complex_separated(&[3.0, 5.0], &[1.0, 6.0])
            .unwrap();

        assert_close(&real, &[17.0 / 6.0, 4.0 / 3.0]);
        assert_close(&imag, &[-5.0 / 6.0, 4.0 / 3.0]);
    }

    #[test]
    fn rejects_short_rhs_for_permuted_input() {
        let matrix =
            FactorizedSparseMatrix::with_maps(vec![SparseSolvePivot::real(1.0)], vec![3], vec![0])
                .unwrap();

        let error = matrix.solve_real(&[1.0, 2.0]).unwrap_err();

        assert_eq!(
            error,
            SparseSolveError::VectorTooShort {
                name: "RHS",
                required: 4,
                actual: 2,
            }
        );
    }

    #[test]
    fn rejects_non_triangular_lower_entry() {
        let error = FactorizedSparseMatrix::from_real_lu(
            [1.0, 1.0],
            vec![vec![(0, 1.0)], vec![]],
            vec![vec![], vec![]],
        )
        .unwrap_err();

        assert_eq!(
            error,
            SparseSolveError::InvalidLowerEntry { pivot: 0, row: 0 }
        );
    }

    #[test]
    fn rejects_complex_matrix_for_real_solve() {
        let matrix = FactorizedSparseMatrix::with_maps(
            vec![SparseSolvePivot::complex(ComplexNumber::new(1.0, 1.0))],
            vec![0],
            vec![0],
        )
        .unwrap();

        let error = matrix.solve_real(&[1.0]).unwrap_err();

        assert_eq!(error, SparseSolveError::ComplexMatrixUsedForRealSolve);
    }
}
