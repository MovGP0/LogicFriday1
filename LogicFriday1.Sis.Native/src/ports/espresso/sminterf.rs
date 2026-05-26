//! Native Rust port of `LogicSynthesis/sis/espresso/sminterf.c`.
//!
//! The C routine translates an Espresso set family into an `sm_matrix`, asks
//! the sparse minimum-cover solver for a unit-cost cover, and translates the
//! selected sparse columns back into an Espresso set. This module keeps that
//! behavior on owned Rust values and leaves C# interop to higher-level facade
//! boundaries.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use crate::ports::mincov::mincov::{MinCoverError, sm_minimum_cover};
use crate::ports::sparse::matrix::SparseMatrix;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspressoSet {
    size: usize,
    elements: BTreeSet<usize>,
}

impl EspressoSet {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            elements: BTreeSet::new(),
        }
    }

    pub fn from_elements<I>(size: usize, elements: I) -> SminterfResult<Self>
    where
        I: IntoIterator<Item = usize>,
    {
        let mut set = Self::new(size);
        for element in elements {
            set.insert(element)?;
        }

        Ok(set)
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn elements(&self) -> &BTreeSet<usize> {
        &self.elements
    }

    pub fn contains(&self, element: usize) -> bool {
        self.elements.contains(&element)
    }

    pub fn insert(&mut self, element: usize) -> SminterfResult<bool> {
        if element >= self.size {
            return Err(SminterfError::ElementOutOfRange {
                element,
                size: self.size,
            });
        }

        Ok(self.elements.insert(element))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspressoSetFamily {
    set_size: usize,
    rows: Vec<EspressoSet>,
}

impl EspressoSetFamily {
    pub fn new(set_size: usize) -> Self {
        Self {
            set_size,
            rows: Vec::new(),
        }
    }

    pub fn from_rows<I, R>(set_size: usize, rows: I) -> SminterfResult<Self>
    where
        I: IntoIterator<Item = R>,
        R: IntoIterator<Item = usize>,
    {
        let mut family = Self::new(set_size);
        for row in rows {
            family.push(EspressoSet::from_elements(set_size, row)?)?;
        }

        Ok(family)
    }

    pub fn set_size(&self) -> usize {
        self.set_size
    }

    pub fn rows(&self) -> &[EspressoSet] {
        &self.rows
    }

    pub fn push(&mut self, row: EspressoSet) -> SminterfResult<()> {
        if row.size() != self.set_size {
            return Err(SminterfError::RowSizeMismatch {
                expected: self.set_size,
                actual: row.size(),
            });
        }

        self.rows.push(row);
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SminterfError {
    ElementOutOfRange { element: usize, size: usize },
    RowSizeMismatch { expected: usize, actual: usize },
    MinimumCover(MinCoverError),
}

impl fmt::Display for SminterfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ElementOutOfRange { element, size } => {
                write!(
                    f,
                    "espresso set element {element} is outside set size {size}"
                )
            }
            Self::RowSizeMismatch { expected, actual } => {
                write!(
                    f,
                    "espresso set-family row has size {actual}; expected {expected}"
                )
            }
            Self::MinimumCover(error) => write!(f, "{error}"),
        }
    }
}

impl Error for SminterfError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::MinimumCover(error) => Some(error),
            _ => None,
        }
    }
}

impl From<MinCoverError> for SminterfError {
    fn from(value: MinCoverError) -> Self {
        Self::MinimumCover(value)
    }
}

pub type SminterfResult<T> = Result<T, SminterfError>;

pub fn do_sm_minimum_cover(family: &EspressoSetFamily) -> SminterfResult<EspressoSet> {
    let matrix = build_sparse_cover_matrix(family);
    let cover = sm_minimum_cover(&matrix, None, true, 0)?;
    EspressoSet::from_elements(family.set_size(), cover.cover)
}

pub fn minimum_cover_from_rows<I, R>(set_size: usize, rows: I) -> SminterfResult<EspressoSet>
where
    I: IntoIterator<Item = R>,
    R: IntoIterator<Item = usize>,
{
    let family = EspressoSetFamily::from_rows(set_size, rows)?;
    do_sm_minimum_cover(&family)
}

pub fn build_sparse_cover_matrix(family: &EspressoSetFamily) -> SparseMatrix {
    let mut matrix = SparseMatrix::new();
    for (row_index, row) in family.rows().iter().enumerate() {
        for element in row.elements() {
            matrix.insert(row_index, *element);
        }
    }

    matrix
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(values: &[usize]) -> BTreeSet<usize> {
        values.iter().copied().collect()
    }

    #[test]
    fn builds_sparse_matrix_with_one_row_per_set_and_column_per_element() {
        let family = EspressoSetFamily::from_rows(8, [vec![0, 2, 7], vec![2, 4]]).unwrap();

        let matrix = build_sparse_cover_matrix(&family);

        assert_eq!(matrix.row_count(), 2);
        assert!(matrix.contains(0, 0));
        assert!(matrix.contains(0, 2));
        assert!(matrix.contains(0, 7));
        assert!(matrix.contains(1, 2));
        assert!(matrix.contains(1, 4));
    }

    #[test]
    fn selects_unit_cost_sparse_cover_columns_as_espresso_set() {
        let cover = minimum_cover_from_rows(6, [vec![0, 1], vec![1, 2], vec![2, 3]]).unwrap();

        assert_eq!(cover.size(), 6);
        assert_eq!(cover.elements(), &set(&[1, 2]));
    }

    #[test]
    fn shared_element_covers_all_rows() {
        let family = EspressoSetFamily::from_rows(4, [vec![0, 3], vec![1, 3], vec![2, 3]]).unwrap();

        let cover = do_sm_minimum_cover(&family).unwrap();

        assert_eq!(cover.elements(), &set(&[3]));
    }

    #[test]
    fn empty_family_returns_empty_cover_with_original_set_size() {
        let family = EspressoSetFamily::new(9);

        let cover = do_sm_minimum_cover(&family).unwrap();

        assert_eq!(cover.size(), 9);
        assert!(cover.elements().is_empty());
    }

    #[test]
    fn rejects_set_elements_outside_the_family_size() {
        assert_eq!(
            EspressoSet::from_elements(3, [0, 3]).unwrap_err(),
            SminterfError::ElementOutOfRange {
                element: 3,
                size: 3,
            }
        );
    }

    #[test]
    fn rejects_rows_from_a_different_sized_family() {
        let mut family = EspressoSetFamily::new(3);
        let row = EspressoSet::from_elements(4, [1, 2]).unwrap();

        assert_eq!(
            family.push(row).unwrap_err(),
            SminterfError::RowSizeMismatch {
                expected: 3,
                actual: 4,
            }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present() {
        let source = include_str!("sminterf.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
    }
}
