//! Native Rust multidimensional array helpers for `sis/map/multi_array.c`.
//!
//! The original SIS helper owns row-major storage plus dimension metadata and
//! exposes access through preprocessor macros. This port keeps that behavior as
//! a typed generic container with checked indexing and explicit diagnostics.

use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct MultiArray<T> {
    max_index: Vec<usize>,
    entries: Vec<T>,
}

impl<T> MultiArray<T> {
    pub fn from_vec(max_index: Vec<usize>, entries: Vec<T>) -> Result<Self, MultiArrayError> {
        let n_entries = checked_entry_count(&max_index)?;
        if entries.len() != n_entries {
            return Err(MultiArrayError::InconsistentObjectSize {
                expected: n_entries,
                actual: entries.len(),
            });
        }

        Ok(Self { max_index, entries })
    }

    pub fn from_fn(
        max_index: Vec<usize>,
        mut init: impl FnMut(usize) -> T,
    ) -> Result<Self, MultiArrayError> {
        let n_entries = checked_entry_count(&max_index)?;
        let entries = (0..n_entries).map(&mut init).collect();

        Ok(Self { max_index, entries })
    }

    pub fn dimension_count(&self) -> usize {
        self.max_index.len()
    }

    pub fn dimensions(&self) -> &[usize] {
        &self.max_index
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn as_slice(&self) -> &[T] {
        &self.entries
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.entries
    }

    pub fn linear_index(&self, indices: &[usize]) -> Result<usize, MultiArrayError> {
        linear_index(&self.max_index, indices)
    }

    pub fn get(&self, indices: &[usize]) -> Result<&T, MultiArrayError> {
        let index = self.linear_index(indices)?;

        Ok(&self.entries[index])
    }

    pub fn get_mut(&mut self, indices: &[usize]) -> Result<&mut T, MultiArrayError> {
        let index = self.linear_index(indices)?;

        Ok(&mut self.entries[index])
    }

    pub fn index2(&self, a: usize, b: usize) -> Result<&T, MultiArrayError> {
        self.get(&[a, b])
    }

    pub fn index2_mut(&mut self, a: usize, b: usize) -> Result<&mut T, MultiArrayError> {
        self.get_mut(&[a, b])
    }

    pub fn index3(&self, a: usize, b: usize, c: usize) -> Result<&T, MultiArrayError> {
        self.get(&[a, b, c])
    }

    pub fn index3_mut(&mut self, a: usize, b: usize, c: usize) -> Result<&mut T, MultiArrayError> {
        self.get_mut(&[a, b, c])
    }

    pub fn index4(&self, a: usize, b: usize, c: usize, d: usize) -> Result<&T, MultiArrayError> {
        self.get(&[a, b, c, d])
    }

    pub fn index4_mut(
        &mut self,
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    ) -> Result<&mut T, MultiArrayError> {
        self.get_mut(&[a, b, c, d])
    }

    pub fn index5(
        &self,
        a: usize,
        b: usize,
        c: usize,
        d: usize,
        e: usize,
    ) -> Result<&T, MultiArrayError> {
        self.get(&[a, b, c, d, e])
    }

    pub fn index5_mut(
        &mut self,
        a: usize,
        b: usize,
        c: usize,
        d: usize,
        e: usize,
    ) -> Result<&mut T, MultiArrayError> {
        self.get_mut(&[a, b, c, d, e])
    }
}

impl<T> MultiArray<T>
where
    T: Clone,
{
    pub fn new(max_index: Vec<usize>, init_value: T) -> Result<Self, MultiArrayError> {
        let n_entries = checked_entry_count(&max_index)?;

        Ok(Self {
            max_index,
            entries: vec![init_value; n_entries],
        })
    }

    pub fn fill(&mut self, value: T) {
        self.entries.fill(value);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MultiArrayError {
    EmptyDimensions,
    InvalidDimension {
        dimension: usize,
        len: usize,
    },
    DimensionCountMismatch {
        expected: usize,
        actual: usize,
    },
    IndexOutOfBounds {
        dimension: usize,
        index: usize,
        len: usize,
    },
    EntryCountOverflow,
    InconsistentObjectSize {
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for MultiArrayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDimensions => write!(
                f,
                "multidim array error: inconsistent object size: at least one dimension is required"
            ),
            Self::InvalidDimension { dimension, len } => write!(
                f,
                "multidim array error: inconsistent object size: dimension {dimension} has length {len}"
            ),
            Self::DimensionCountMismatch { expected, actual } => write!(
                f,
                "multidim array error: inconsistent object size: expected {expected} indices but got {actual}"
            ),
            Self::IndexOutOfBounds {
                dimension,
                index,
                len,
            } => write!(
                f,
                "multidim array error: index out of bounds: index {index} for dimension {dimension} with length {len}"
            ),
            Self::EntryCountOverflow => write!(
                f,
                "multidim array error: inconsistent object size: entry count overflow"
            ),
            Self::InconsistentObjectSize { expected, actual } => write!(
                f,
                "multidim array error: inconsistent object size: expected {expected} entries but got {actual}"
            ),
        }
    }
}

impl Error for MultiArrayError {}

pub fn linear_index(max_index: &[usize], indices: &[usize]) -> Result<usize, MultiArrayError> {
    checked_entry_count(max_index)?;
    if indices.len() != max_index.len() {
        return Err(MultiArrayError::DimensionCountMismatch {
            expected: max_index.len(),
            actual: indices.len(),
        });
    }

    let mut offset = 0usize;
    for (dimension, (&index, &len)) in indices.iter().zip(max_index.iter()).enumerate() {
        if index >= len {
            return Err(MultiArrayError::IndexOutOfBounds {
                dimension,
                index,
                len,
            });
        }

        offset = offset
            .checked_mul(len)
            .and_then(|value| value.checked_add(index))
            .ok_or(MultiArrayError::EntryCountOverflow)?;
    }

    Ok(offset)
}

fn checked_entry_count(max_index: &[usize]) -> Result<usize, MultiArrayError> {
    if max_index.is_empty() {
        return Err(MultiArrayError::EmptyDimensions);
    }

    let mut count = 1usize;
    for (dimension, &len) in max_index.iter().enumerate() {
        if len == 0 {
            return Err(MultiArrayError::InvalidDimension { dimension, len });
        }

        count = count
            .checked_mul(len)
            .ok_or(MultiArrayError::EntryCountOverflow)?;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_initialized_row_major_storage() {
        let array = MultiArray::new(vec![2, 3], 7).unwrap();

        assert_eq!(array.dimension_count(), 2);
        assert_eq!(array.dimensions(), &[2, 3]);
        assert_eq!(array.entry_count(), 6);
        assert_eq!(array.as_slice(), &[7, 7, 7, 7, 7, 7]);
        assert_eq!(*array.index2(1, 2).unwrap(), 7);
    }

    #[test]
    fn computes_c_macro_row_major_offsets() {
        assert_eq!(linear_index(&[2, 3], &[1, 2]).unwrap(), 5);
        assert_eq!(linear_index(&[2, 3, 4], &[1, 2, 3]).unwrap(), 23);
        assert_eq!(linear_index(&[2, 3, 4, 5], &[1, 2, 3, 4]).unwrap(), 119);
        assert_eq!(
            linear_index(&[2, 3, 4, 5, 6], &[1, 2, 3, 4, 5]).unwrap(),
            719
        );
    }

    #[test]
    fn from_fn_initializes_by_linear_offset() {
        let mut array = MultiArray::from_fn(vec![2, 2, 2], |index| index * 10).unwrap();

        assert_eq!(*array.index3(1, 0, 1).unwrap(), 50);
        *array.index3_mut(1, 0, 1).unwrap() = 99;
        assert_eq!(array.as_slice(), &[0, 10, 20, 30, 40, 99, 60, 70]);
    }

    #[test]
    fn fill_replaces_every_entry() {
        let mut array = MultiArray::from_vec(vec![2, 2], vec![1, 2, 3, 4]).unwrap();

        array.fill(9);

        assert_eq!(array.as_slice(), &[9, 9, 9, 9]);
    }

    #[test]
    fn rejects_bad_dimensions_and_lengths() {
        assert_eq!(
            MultiArray::<u8>::new(Vec::new(), 0).unwrap_err(),
            MultiArrayError::EmptyDimensions
        );
        assert_eq!(
            MultiArray::<u8>::new(vec![2, 0], 0).unwrap_err(),
            MultiArrayError::InvalidDimension {
                dimension: 1,
                len: 0,
            }
        );
        assert_eq!(
            MultiArray::from_vec(vec![2, 2], vec![1, 2, 3]).unwrap_err(),
            MultiArrayError::InconsistentObjectSize {
                expected: 4,
                actual: 3,
            }
        );
    }

    #[test]
    fn rejects_wrong_rank_and_out_of_bounds_indices() {
        let array = MultiArray::new(vec![2, 3], 0).unwrap();

        assert_eq!(
            array.linear_index(&[1]).unwrap_err(),
            MultiArrayError::DimensionCountMismatch {
                expected: 2,
                actual: 1,
            }
        );
        assert_eq!(
            array.linear_index(&[1, 3]).unwrap_err(),
            MultiArrayError::IndexOutOfBounds {
                dimension: 1,
                index: 3,
                len: 3,
            }
        );
    }

    #[test]
    fn detects_entry_count_overflow() {
        assert_eq!(
            MultiArray::<u8>::new(vec![usize::MAX, 2], 0).unwrap_err(),
            MultiArrayError::EntryCountOverflow
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("multi_array.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
