//! Native Rust array helpers for the SIS array package.
//!
//! The C package stores fixed-width objects behind macros. This port exposes
//! the same container behavior as a typed Rust collection with checked access.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SisArray<T>
{
    entries: Vec<T>,
    capacity: usize,
}

impl<T> SisArray<T>
{
    pub fn from_vec(entries: Vec<T>) -> Self
    {
        let capacity = entries.len();

        Self { entries, capacity }
    }

    pub fn len(&self) -> usize
    {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.entries.is_empty()
    }

    pub fn capacity(&self) -> usize
    {
        self.capacity
    }

    pub fn as_slice(&self) -> &[T]
    {
        &self.entries
    }

    pub fn as_mut_slice(&mut self) -> &mut [T]
    {
        &mut self.entries
    }

    pub fn into_vec(self) -> Vec<T>
    {
        self.entries
    }

    pub fn fetch(&self, index: usize) -> Result<&T, SisArrayError>
    {
        self.entries
            .get(index)
            .ok_or(SisArrayError::IndexOutOfBounds {
                index,
                len: self.entries.len(),
            })
    }

    pub fn fetch_mut(&mut self, index: usize) -> Result<&mut T, SisArrayError>
    {
        let len = self.entries.len();
        self.entries
            .get_mut(index)
            .ok_or(SisArrayError::IndexOutOfBounds { index, len })
    }

    pub fn fetch_last(&self) -> Result<&T, SisArrayError>
    {
        let len = self.entries.len();
        self.entries.last().ok_or(SisArrayError::IndexOutOfBounds {
            index: 0,
            len,
        })
    }

    pub fn append(&mut self, other: &Self)
    where
        T: Clone,
    {
        self.ensure_capacity(self.entries.len() + other.entries.len());
        self.entries.extend_from_slice(&other.entries);
    }

    pub fn join(&self, other: &Self) -> Self
    where
        T: Clone,
    {
        let mut joined = Self::with_capacity_for_len(self.entries.len() + other.entries.len());
        joined.entries.extend_from_slice(&self.entries);
        joined.entries.extend_from_slice(&other.entries);
        joined
    }

    pub fn data(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.entries.clone()
    }

    pub fn sort_by(&mut self, compare: impl FnMut(&T, &T) -> Ordering)
    {
        self.entries.sort_by(compare);
    }

    pub fn uniq_by(&mut self, compare: impl FnMut(&T, &T) -> Ordering)
    {
        self.uniq_by_with_removed(compare, drop);
    }

    pub fn uniq_by_with_removed(
        &mut self,
        mut compare: impl FnMut(&T, &T) -> Ordering,
        mut removed: impl FnMut(T),
    )
    {
        let old_capacity = self.capacity;
        let old_entries = std::mem::take(&mut self.entries);
        let mut iter = old_entries.into_iter().peekable();
        let mut unique = Vec::with_capacity(old_capacity);

        while let Some(entry) = iter.next()
        {
            if let Some(next) = iter.peek()
            {
                if compare(&entry, next) == Ordering::Equal
                {
                    removed(entry);
                    continue;
                }
            }

            unique.push(entry);
        }

        self.entries = unique;
    }

    fn with_capacity_for_len(len: usize) -> Self
    {
        Self {
            entries: Vec::with_capacity(len),
            capacity: len,
        }
    }

    fn ensure_capacity(&mut self, required: usize)
    {
        if self.capacity >= required
        {
            return;
        }

        let target = if self.capacity == 0
        {
            required
        }
        else
        {
            self.capacity.saturating_mul(2).max(required)
        };

        if target > self.entries.capacity()
        {
            self.entries.reserve_exact(target - self.entries.capacity());
        }

        self.capacity = target;
    }
}

impl<T> Default for SisArray<T>
where
    T: Default,
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl<T> SisArray<T>
where
    T: Default,
{
    pub fn new() -> Self
    {
        Self::with_capacity(0)
    }

    pub fn with_capacity(capacity: usize) -> Self
    {
        Self {
            entries: Vec::with_capacity(initial_capacity(capacity)),
            capacity: initial_capacity(capacity),
        }
    }

    pub fn insert(&mut self, index: usize, datum: T)
    {
        self.ensure_capacity(index + 1);

        if index < self.entries.len()
        {
            self.entries[index] = datum;
        }
        else
        {
            self.entries.resize_with(index, T::default);
            self.entries.push(datum);
        }
    }

    pub fn insert_last(&mut self, datum: T)
    {
        self.insert(self.entries.len(), datum);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SisArrayError
{
    IndexOutOfBounds
    {
        index: usize,
        len: usize,
    },
}

impl fmt::Display for SisArrayError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::IndexOutOfBounds { index, len } =>
            {
                if *len == 0
                {
                    write!(f, "array error: fetch index {index} not in an empty array")
                }
                else
                {
                    write!(
                        f,
                        "array error: fetch index {index} not in [0,{}]",
                        len - 1
                    )
                }
            }
        }
    }
}

impl Error for SisArrayError {}

fn initial_capacity(requested: usize) -> usize
{
    requested.max(3)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn allocation_uses_c_minimum_capacity_without_entries()
    {
        let array = SisArray::<i32>::with_capacity(0);

        assert_eq!(array.len(), 0);
        assert!(array.capacity() >= 3);
        assert!(array.is_empty());
    }

    #[test]
    fn insert_extends_length_and_default_fills_gaps()
    {
        let mut array = SisArray::<i32>::new();

        array.insert(2, 7);

        assert_eq!(array.as_slice(), &[0, 0, 7]);
        assert_eq!(array.len(), 3);

        array.insert(1, 5);

        assert_eq!(array.as_slice(), &[0, 5, 7]);
    }

    #[test]
    fn append_join_and_data_clone_preserve_order()
    {
        let mut first = SisArray::from_vec(vec![1, 2]);
        let second = SisArray::from_vec(vec![3, 4]);

        first.append(&second);

        assert_eq!(first.as_slice(), &[1, 2, 3, 4]);
        assert_eq!(first.data(), vec![1, 2, 3, 4]);
        assert_eq!(second.as_slice(), &[3, 4]);
        assert_eq!(first.join(&second).as_slice(), &[1, 2, 3, 4, 3, 4]);
    }

    #[test]
    fn sort_and_uniq_keep_last_entry_from_each_equal_run()
    {
        #[derive(Clone, Debug, Eq, PartialEq)]
        struct Item
        {
            key: i32,
            label: &'static str,
        }

        let mut array = SisArray::from_vec(vec![
            Item { key: 2, label: "d" },
            Item { key: 1, label: "a" },
            Item { key: 1, label: "b" },
            Item { key: 2, label: "c" },
        ]);

        array.sort_by(|left, right| left.key.cmp(&right.key));

        let mut removed = Vec::new();
        array.uniq_by_with_removed(
            |left, right| left.key.cmp(&right.key),
            |item| removed.push(item.label),
        );

        let labels: Vec<_> = array.as_slice().iter().map(|item| item.label).collect();

        assert_eq!(labels, vec!["b", "c"]);
        assert_eq!(removed, vec!["a", "d"]);
    }

    #[test]
    fn fetch_reports_bounds_errors()
    {
        let array = SisArray::from_vec(vec![10, 20]);

        assert_eq!(array.fetch(1), Ok(&20));
        assert_eq!(
            array.fetch(2).unwrap_err(),
            SisArrayError::IndexOutOfBounds { index: 2, len: 2 }
        );
    }
}
