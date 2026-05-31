//! Native Rust port of `sis/st/st.c`.
//!
//! The original SIS `st` package is a small chained hash table parameterized by
//! C function pointers for comparison and hashing. This port keeps the same
//! observable table behavior, including direct duplicate insertion and deletion
//! during traversal, but exposes it as owned Rust data instead of raw
//! `char *`/result-parameter APIs.

use std::fmt;

pub const DEFAULT_MAX_DENSITY: usize = 5;
pub const DEFAULT_INIT_TABLE_SIZE: usize = 11;
pub const DEFAULT_GROW_FACTOR: f64 = 2.0;
pub const DEFAULT_REORDER: bool = false;

type Compare<K> = dyn Fn(&K, &K) -> bool;
type Hash<K> = dyn Fn(&K, usize) -> usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForeachControl {
    Continue,
    Stop,
    Delete,
}

struct Entry<K, V> {
    key: K,
    value: V,
}

pub struct StTable<K, V> {
    compare: Box<Compare<K>>,
    hash: Box<Hash<K>>,
    bins: Vec<Vec<Entry<K, V>>>,
    entries: usize,
    max_density: usize,
    grow_factor: f64,
    reorder: bool,
}

impl<K, V> fmt::Debug for StTable<K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StTable")
            .field("bins", &self.bins)
            .field("entries", &self.entries)
            .field("max_density", &self.max_density)
            .field("grow_factor", &self.grow_factor)
            .field("reorder", &self.reorder)
            .finish_non_exhaustive()
    }
}

impl<K, V> fmt::Debug for Entry<K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Entry")
            .field(&self.key)
            .field(&self.value)
            .finish()
    }
}

impl<K, V> StTable<K, V> {
    pub fn with_params(
        compare: impl Fn(&K, &K) -> bool + 'static,
        hash: impl Fn(&K, usize) -> usize + 'static,
        size: usize,
        max_density: usize,
        grow_factor: f64,
        reorder: bool,
    ) -> Self {
        let size = size.max(1);
        let mut bins = Vec::with_capacity(size);
        bins.resize_with(size, Vec::new);

        Self {
            compare: Box::new(compare),
            hash: Box::new(hash),
            bins,
            entries: 0,
            max_density,
            grow_factor,
            reorder,
        }
    }

    pub fn new(
        compare: impl Fn(&K, &K) -> bool + 'static,
        hash: impl Fn(&K, usize) -> usize + 'static,
    ) -> Self {
        Self::with_params(
            compare,
            hash,
            DEFAULT_INIT_TABLE_SIZE,
            DEFAULT_MAX_DENSITY,
            DEFAULT_GROW_FACTOR,
            DEFAULT_REORDER,
        )
    }

    pub fn len(&self) -> usize {
        self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries == 0
    }

    pub fn bin_count(&self) -> usize {
        self.bins.len()
    }

    pub fn contains_key(&mut self, key: &K) -> bool {
        self.lookup(key).is_some()
    }

    pub fn lookup(&mut self, key: &K) -> Option<&V> {
        let (bin_index, entry_index) = self.find_position(key);
        let entry_index = entry_index?;
        self.promote_if_requested(bin_index, entry_index);
        let entry_index = self.resolved_entry_index(entry_index);
        self.bins[bin_index]
            .get(entry_index)
            .map(|entry| &entry.value)
    }

    pub fn lookup_mut(&mut self, key: &K) -> Option<&mut V> {
        let (bin_index, entry_index) = self.find_position(key);
        let entry_index = entry_index?;
        self.promote_if_requested(bin_index, entry_index);
        let entry_index = self.resolved_entry_index(entry_index);
        self.bins[bin_index]
            .get_mut(entry_index)
            .map(|entry| &mut entry.value)
    }

    pub fn insert(&mut self, key: K, value: V) -> bool {
        let (bin_index, entry_index) = self.find_position(&key);

        if let Some(entry_index) = entry_index {
            self.promote_if_requested(bin_index, entry_index);
            let entry_index = self.resolved_entry_index(entry_index);
            self.bins[bin_index][entry_index].value = value;
            true
        } else {
            self.add_direct_at(bin_index, key, value);
            false
        }
    }

    pub fn add_direct(&mut self, key: K, value: V) {
        let bin_index = self.hash_key(&key);
        self.add_direct_at(bin_index, key, value);
    }

    pub fn find_or_insert_with(&mut self, key: K, value: impl FnOnce() -> V) -> (bool, &mut V) {
        let (bin_index, entry_index) = self.find_position(&key);

        if let Some(entry_index) = entry_index {
            self.promote_if_requested(bin_index, entry_index);
            let entry_index = self.resolved_entry_index(entry_index);
            (true, &mut self.bins[bin_index][entry_index].value)
        } else {
            let bin_index = self.add_direct_at(bin_index, key, value());
            (false, &mut self.bins[bin_index][0].value)
        }
    }

    pub fn find(&mut self, key: &K) -> Option<&mut V> {
        self.lookup_mut(key)
    }

    pub fn delete(&mut self, key: &K) -> Option<(K, V)> {
        let (bin_index, entry_index) = self.find_position(key);
        let entry_index = entry_index?;
        self.entries -= 1;
        let entry = self.bins[bin_index].remove(entry_index);
        Some((entry.key, entry.value))
    }

    pub fn foreach(&mut self, mut visit: impl FnMut(&K, &mut V) -> ForeachControl) -> bool {
        let mut bin_index = 0;
        while bin_index < self.bins.len() {
            let mut entry_index = 0;
            while entry_index < self.bins[bin_index].len() {
                let entry = &mut self.bins[bin_index][entry_index];
                match visit(&entry.key, &mut entry.value) {
                    ForeachControl::Continue => entry_index += 1,
                    ForeachControl::Stop => return false,
                    ForeachControl::Delete => {
                        self.bins[bin_index].remove(entry_index);
                        self.entries -= 1;
                    }
                }
            }
            bin_index += 1;
        }
        true
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.bins
            .iter()
            .flat_map(|bin| bin.iter().map(|entry| (&entry.key, &entry.value)))
    }

    fn find_position(&self, key: &K) -> (usize, Option<usize>) {
        let bin_index = self.hash_key(key);
        let entry_index = self.bins[bin_index]
            .iter()
            .position(|entry| (self.compare)(key, &entry.key));

        (bin_index, entry_index)
    }

    fn add_direct_at(&mut self, mut bin_index: usize, key: K, value: V) -> usize {
        if self.entries / self.bins.len() >= self.max_density {
            self.rehash();
            bin_index = self.hash_key(&key);
        }

        self.bins[bin_index].insert(0, Entry { key, value });
        self.entries += 1;
        bin_index
    }

    fn promote_if_requested(&mut self, bin_index: usize, entry_index: usize) {
        if self.reorder && entry_index != 0 {
            let entry = self.bins[bin_index].remove(entry_index);
            self.bins[bin_index].insert(0, entry);
        }
    }

    fn resolved_entry_index(&self, entry_index: usize) -> usize {
        if self.reorder { 0 } else { entry_index }
    }

    fn rehash(&mut self) {
        let old_bins = std::mem::take(&mut self.bins);
        let mut new_size = ((old_bins.len() as f64) * self.grow_factor) as usize;
        if new_size == 0 {
            new_size = 1;
        }
        if new_size % 2 == 0 {
            new_size += 1;
        }

        self.bins = Vec::with_capacity(new_size);
        self.bins.resize_with(new_size, Vec::new);
        self.entries = 0;

        for entry in old_bins.into_iter().flatten() {
            let bin_index = self.hash_key(&entry.key);
            self.bins[bin_index].insert(0, entry);
            self.entries += 1;
        }
    }

    fn hash_key(&self, key: &K) -> usize {
        (self.hash)(key, self.bins.len()) % self.bins.len()
    }
}

impl<K, V> StTable<K, V>
where
    K: Clone,
    V: Clone,
{
    pub fn copy_with(
        &self,
        compare: impl Fn(&K, &K) -> bool + 'static,
        hash: impl Fn(&K, usize) -> usize + 'static,
    ) -> Self {
        let mut copy = Self::with_params(
            compare,
            hash,
            self.bins.len(),
            self.max_density,
            self.grow_factor,
            self.reorder,
        );

        copy.bins = self
            .bins
            .iter()
            .map(|bin| {
                bin.iter()
                    .map(|entry| Entry {
                        key: entry.key.clone(),
                        value: entry.value.clone(),
                    })
                    .collect()
            })
            .collect();
        copy.entries = self.entries;
        copy
    }
}

pub fn string_table<V>() -> StTable<String, V> {
    StTable::new(
        |left, right| left == right,
        |key: &String, size| strhash(key, size),
    )
}

pub fn number_table<V>() -> StTable<isize, V> {
    StTable::new(|left, right| left == right, |key, size| numhash(*key, size))
}

pub fn pointer_table<V>() -> StTable<usize, V> {
    StTable::new(|left, right| left == right, |key, size| ptrhash(*key, size))
}

pub fn strhash(string: &str, modulus: usize) -> usize {
    assert!(modulus > 0, "hash modulus must be non-zero");

    let mut value = 0i32;
    for byte in string.bytes() {
        value = value.wrapping_mul(997).wrapping_add(byte as i32);
    }

    value.unsigned_abs() as usize % modulus
}

pub fn numhash(value: isize, modulus: usize) -> usize {
    assert!(modulus > 0, "hash modulus must be non-zero");
    value.unsigned_abs() % modulus
}

pub fn ptrhash(value: usize, modulus: usize) -> usize {
    assert!(modulus > 0, "hash modulus must be non-zero");
    (value >> 2) % modulus
}

pub fn numcmp<T: Eq>(left: &T, right: &T) -> i32 {
    i32::from(left != right)
}

pub fn ptrcmp<T: Eq>(left: &T, right: &T) -> i32 {
    numcmp(left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_replaces_and_reports_membership() {
        let mut table = string_table();

        assert!(!table.insert("a".to_owned(), 1));
        assert_eq!(table.lookup(&"a".to_owned()), Some(&1));
        assert!(table.insert("a".to_owned(), 2));
        assert_eq!(table.lookup(&"a".to_owned()), Some(&2));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn add_direct_keeps_duplicate_entries_like_c_st_add_direct() {
        let mut table = StTable::with_params(
            |left: &i32, right: &i32| left == right,
            |key: &i32, size| key.unsigned_abs() as usize % size,
            3,
            DEFAULT_MAX_DENSITY,
            DEFAULT_GROW_FACTOR,
            false,
        );

        table.add_direct(7, "old");
        table.add_direct(7, "new");

        assert_eq!(table.len(), 2);
        assert_eq!(table.lookup(&7), Some(&"new"));
        assert_eq!(table.delete(&7), Some((7, "new")));
        assert_eq!(table.lookup(&7), Some(&"old"));
    }

    #[test]
    fn non_reordering_tables_return_the_matched_collision_entry() {
        let mut table = StTable::with_params(
            |left: &i32, right: &i32| left == right,
            |_key, _size| 0,
            3,
            DEFAULT_MAX_DENSITY,
            DEFAULT_GROW_FACTOR,
            false,
        );

        table.insert(1, "one");
        table.insert(2, "two");

        assert_eq!(table.lookup(&1), Some(&"one"));
        assert!(table.insert(1, "updated"));
        assert_eq!(table.lookup(&1), Some(&"updated"));
        assert_eq!(table.lookup(&2), Some(&"two"));
    }

    #[test]
    fn find_or_insert_returns_existing_slot_or_new_slot() {
        let mut table = number_table();

        let (found, value) = table.find_or_insert_with(3, || 10);
        assert!(!found);
        *value = 11;

        let (found, value) = table.find_or_insert_with(3, || 12);
        assert!(found);
        assert_eq!(*value, 11);
    }

    #[test]
    fn foreach_can_stop_or_delete_entries() {
        let mut table = number_table();
        for key in 0..5 {
            table.insert(key, key * 10);
        }

        let completed = table.foreach(|key, _| {
            if *key == 2 {
                ForeachControl::Stop
            } else {
                ForeachControl::Continue
            }
        });
        assert!(!completed);

        assert!(table.foreach(|key, _| {
            if *key % 2 == 0 {
                ForeachControl::Delete
            } else {
                ForeachControl::Continue
            }
        }));

        assert_eq!(table.len(), 2);
        assert!(table.lookup(&1).is_some());
        assert!(table.lookup(&3).is_some());
    }

    #[test]
    fn rehash_uses_odd_growth_and_preserves_entries() {
        let mut table = StTable::with_params(
            |left: &usize, right: &usize| left == right,
            |key: &usize, size| *key % size,
            1,
            1,
            2.0,
            false,
        );

        table.insert(0, "a");
        table.insert(1, "b");

        assert_eq!(table.bin_count(), 3);
        assert_eq!(table.lookup(&0), Some(&"a"));
        assert_eq!(table.lookup(&1), Some(&"b"));
    }

    #[test]
    fn hash_helpers_match_st_c_formulas() {
        assert_eq!(strhash("abc", 101), 68);
        assert_eq!(numhash(-12, 5), 2);
        assert_eq!(ptrhash(0x20, 7), 1);
        assert_eq!(numcmp(&1, &1), 0);
        assert_eq!(ptrcmp(&1, &2), 1);
    }
}
