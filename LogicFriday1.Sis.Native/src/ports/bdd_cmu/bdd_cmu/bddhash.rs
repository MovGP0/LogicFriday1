//! Native Rust hash-table helpers for the CMU BDD package.
//!
//! The C implementation stores arbitrary byte payloads in chained buckets keyed
//! by BDD pointer identity. This port keeps the observable table behavior while
//! representing payloads as typed Rust values owned by the table.

use std::fmt;

const INITIAL_SIZE_INDEX: usize = 10;
const TABLE_SIZES: &[usize] = &[
    1, 2, 3, 7, 13, 23, 59, 113, 241, 503, 1019, 2039, 4091, 8179, 11587, 16369, 23143, 32749,
    46349, 65521, 92683, 131063, 185363, 262139, 330287, 416147, 524269, 660557, 832253, 1048571,
    1321109, 1664501, 2097143, 2642201, 3328979, 4194287, 5284393, 6657919, 8388593, 10568797,
    13315831, 16777199, 33554393, 67108859, 134217689, 268435399, 536870879, 1073741789,
    2147483629,
];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddHashKey(usize);

impl BddHashKey {
    pub const fn new(raw: usize) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> usize {
        self.0
    }
}

impl From<usize> for BddHashKey {
    fn from(value: usize) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddHashError {
    TableSizeExhausted { size_index: usize },
}

impl fmt::Display for BddHashError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TableSizeExhausted { size_index } => write!(
                formatter,
                "CMU BDD hash table size index {size_index} exceeds the legacy size table"
            ),
        }
    }
}

impl std::error::Error for BddHashError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HashEntry<T> {
    key: BddHashKey,
    value: T,
}

#[derive(Clone, Debug)]
pub struct BddHashTable<T> {
    buckets: Vec<Vec<HashEntry<T>>>,
    size_index: usize,
    entries: usize,
}

impl<T> BddHashTable<T> {
    pub fn new() -> Self {
        Self::with_size_index(INITIAL_SIZE_INDEX)
            .expect("initial CMU BDD hash table size index is valid")
    }

    pub fn with_size_index(size_index: usize) -> Result<Self, BddHashError> {
        let size = table_size(size_index)?;

        Ok(Self {
            buckets: (0..size).map(|_| Vec::new()).collect(),
            size_index,
            entries: 0,
        })
    }

    pub fn insert<K>(&mut self, key: K, value: T) -> Result<(), BddHashError>
    where
        K: Into<BddHashKey>,
    {
        let key = key.into();
        let bucket = reduce_hash(key, self.buckets.len());
        self.buckets[bucket].insert(0, HashEntry { key, value });
        self.entries += 1;

        if (self.buckets.len() << 2) < self.entries {
            self.rehash()?;
        }

        Ok(())
    }

    pub fn lookup<K>(&self, key: K) -> Option<&T>
    where
        K: Into<BddHashKey>,
    {
        let key = key.into();
        let bucket = reduce_hash(key, self.buckets.len());

        self.buckets[bucket]
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| &entry.value)
    }

    pub fn lookup_mut<K>(&mut self, key: K) -> Option<&mut T>
    where
        K: Into<BddHashKey>,
    {
        let key = key.into();
        let bucket = reduce_hash(key, self.buckets.len());

        self.buckets[bucket]
            .iter_mut()
            .find(|entry| entry.key == key)
            .map(|entry| &mut entry.value)
    }

    pub fn len(&self) -> usize {
        self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries == 0
    }

    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    pub fn size_index(&self) -> usize {
        self.size_index
    }

    pub fn load_factor(&self) -> f64 {
        self.entries as f64 / self.buckets.len() as f64
    }

    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }

        self.entries = 0;
    }

    fn rehash(&mut self) -> Result<(), BddHashError> {
        let next_index = self.size_index + 1;
        let next_size = table_size(next_index)?;
        let mut new_buckets: Vec<Vec<HashEntry<T>>> = (0..next_size).map(|_| Vec::new()).collect();

        for bucket in &mut self.buckets {
            for entry in bucket.drain(..) {
                let new_bucket = reduce_hash(entry.key, next_size);
                new_buckets[new_bucket].insert(0, entry);
            }
        }

        self.buckets = new_buckets;
        self.size_index = next_index;
        Ok(())
    }
}

impl<T> Default for BddHashTable<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn table_size(size_index: usize) -> Result<usize, BddHashError> {
    TABLE_SIZES
        .get(size_index)
        .copied()
        .ok_or(BddHashError::TableSizeExhausted { size_index })
}

pub fn reduce_hash(key: BddHashKey, size: usize) -> usize {
    key.raw() % size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_table_uses_legacy_initial_size() {
        let table = BddHashTable::<usize>::new();

        assert_eq!(table.size_index(), 10);
        assert_eq!(table.bucket_count(), 1019);
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());
    }

    #[test]
    fn insert_and_lookup_return_associated_payload() {
        let mut table = BddHashTable::new();

        table.insert(0x1200usize, "node data").unwrap();

        assert_eq!(table.lookup(0x1200usize), Some(&"node data"));
        assert_eq!(table.lookup(0x1201usize), None);
    }

    #[test]
    fn duplicate_key_lookup_returns_most_recent_entry() {
        let mut table = BddHashTable::new();

        table.insert(42usize, "old").unwrap();
        table.insert(42usize, "new").unwrap();

        assert_eq!(table.len(), 2);
        assert_eq!(table.lookup(42usize), Some(&"new"));
    }

    #[test]
    fn lookup_mut_updates_most_recent_entry_only() {
        let mut table = BddHashTable::new();

        table.insert(7usize, 1).unwrap();
        table.insert(7usize, 2).unwrap();
        *table.lookup_mut(7usize).unwrap() = 3;

        assert_eq!(table.lookup(7usize), Some(&3));
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn grows_when_entries_exceed_four_times_bucket_count() {
        let mut table = BddHashTable::with_size_index(3).unwrap();
        let initial_buckets = table.bucket_count();

        for key in 0..=(initial_buckets * 4) {
            table.insert(key, key).unwrap();
        }

        assert_eq!(initial_buckets, 7);
        assert_eq!(table.size_index(), 4);
        assert_eq!(table.bucket_count(), 13);

        for key in 0..=(initial_buckets * 4) {
            assert_eq!(table.lookup(key), Some(&key));
        }
    }

    #[test]
    fn clear_drops_entries_but_keeps_allocated_table_size() {
        let mut table = BddHashTable::with_size_index(3).unwrap();
        table.insert(1usize, "one").unwrap();
        table.insert(2usize, "two").unwrap();

        table.clear();

        assert!(table.is_empty());
        assert_eq!(table.bucket_count(), 7);
        assert_eq!(table.lookup(1usize), None);
    }

    #[test]
    fn reports_size_table_exhaustion() {
        assert_eq!(
            table_size(TABLE_SIZES.len()),
            Err(BddHashError::TableSizeExhausted {
                size_index: TABLE_SIZES.len()
            })
        );
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("bddhash.rs");
        let legacy_export = concat!("no", "_", "mangle");
        let tracking_prefix = concat!("REQUIRED", "_");
        let dependency_type = concat!("Port", "Dependency");
        let bead_token = concat!("bead", "_id");
        let source_token = concat!("source", "_file");
        let bead_prefix = concat!("Logic", "Friday1", "-", "8j8");

        assert!(!source.contains(legacy_export));
        assert!(!source.contains("extern \"C\""));
        assert!(!source.contains(tracking_prefix));
        assert!(!source.contains(dependency_type));
        assert!(!source.contains(bead_token));
        assert!(!source.contains(source_token));
        assert!(!source.contains(bead_prefix));
    }
}
