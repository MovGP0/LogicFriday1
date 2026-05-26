//! Native Rust operation-cache support for the CMU BDD package.
//!
//! The original `bddcache.c` stores tagged cache records in two-entry bins,
//! promotes LRU hits, supports built-in and user cache tags, and purges entries
//! through per-tag callbacks. This port keeps the same cache behavior while
//! replacing tagged pointer storage and function-pointer sentinels with
//! Rust-owned values and callbacks.

use std::fmt;

pub type CacheTag = usize;
pub type CacheWord = isize;

pub const CACHE_TYPE_ITE: CacheTag = 0;
pub const CACHE_TYPE_TWO: CacheTag = 1;
pub const CACHE_TYPE_ONEDATA: CacheTag = 2;
pub const CACHE_TYPE_TWODATA: CacheTag = 3;
pub const CACHE_TYPE_USER1: CacheTag = 4;
pub const USER_ENTRY_TYPES: usize = 32;
pub const OP_RELPROD: CacheWord = 0x1000;
pub const OP_QNT: CacheWord = 0x2000;
pub const OP_SUBST: CacheWord = 0x3000;

const INITIAL_SIZE_INDEX: usize = 13;
const TABLE_SIZES: [usize; 49] = [
    1, 2, 3, 7, 13, 23, 59, 113, 241, 503, 1019, 2039, 4091, 8179, 11587, 16369, 23143, 32749,
    46349, 65521, 92683, 131063, 185363, 262139, 330287, 416147, 524269, 660557, 832253, 1048571,
    1321109, 1664501, 2097143, 2642201, 3328979, 4194287, 5284393, 6657919, 8388593, 10568797,
    13315831, 16777199, 33554393, 67108859, 134217689, 268435399, 536870879, 1073741789,
    2147483629,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CacheArity {
    One,
    Two,
    Three,
}

impl CacheArity {
    fn rehash(self, entry: &CacheEntry) -> CacheWord {
        match self {
            Self::One => hash1(entry.slots[0]),
            Self::Two => hash2(entry.slots[0], entry.slots[1]),
            Self::Three => hash3(entry.slots[0], entry.slots[1], entry.slots[2]),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CacheEntry {
    tag: CacheTag,
    slots: [CacheWord; 4],
}

impl CacheEntry {
    pub const fn tag(&self) -> CacheTag {
        self.tag
    }

    pub const fn slots(&self) -> [CacheWord; 4] {
        self.slots
    }
}

#[derive(Clone, Copy)]
pub struct CacheCallbacks {
    pub arity: CacheArity,
    pub should_collect: fn(&CacheEntry) -> bool,
    pub purge: Option<fn(&mut CacheEntry)>,
    pub on_return: Option<fn(&mut CacheEntry)>,
    pub should_flush: Option<fn(&CacheEntry, CacheWord) -> bool>,
}

impl fmt::Debug for CacheCallbacks {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CacheCallbacks")
            .field("arity", &self.arity)
            .field("has_purge", &self.purge.is_some())
            .field("has_on_return", &self.on_return.is_some())
            .field("has_should_flush", &self.should_flush.is_some())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CacheStats {
    pub entries: usize,
    pub lookups: usize,
    pub hits: usize,
    pub inserts: usize,
    pub collisions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CacheError {
    InvalidArity(usize),
    InvalidTag(CacheTag),
    InvalidSizeIndex(usize),
    NoUserTagsAvailable,
    UnallocatedUserTag(CacheTag),
}

impl fmt::Display for CacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArity(arity) => write!(formatter, "illegal cache arity {arity}"),
            Self::InvalidTag(tag) => write!(formatter, "invalid cache tag {tag}"),
            Self::InvalidSizeIndex(index) => write!(formatter, "invalid cache size index {index}"),
            Self::NoUserTagsAvailable => formatter.write_str("no user cache tags available"),
            Self::UnallocatedUserTag(tag) => write!(formatter, "unallocated user cache tag {tag}"),
        }
    }
}

impl std::error::Error for CacheError {}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct CacheBin {
    entries: [Option<CacheEntry>; 2],
}

#[derive(Clone, Debug)]
pub struct OperationCache {
    size_index: usize,
    bins: Vec<CacheBin>,
    callbacks: Vec<Option<CacheCallbacks>>,
    cache_ratio: usize,
    cache_level: usize,
    stats: CacheStats,
}

impl OperationCache {
    pub fn new() -> Self {
        let mut cache = Self {
            size_index: INITIAL_SIZE_INDEX,
            bins: vec![CacheBin::default(); TABLE_SIZES[INITIAL_SIZE_INDEX]],
            callbacks: vec![None; CACHE_TYPE_USER1 + USER_ENTRY_TYPES],
            cache_ratio: 4,
            cache_level: 0,
            stats: CacheStats::default(),
        };
        cache.callbacks[CACHE_TYPE_ITE] = Some(CacheCallbacks {
            arity: CacheArity::Three,
            should_collect: cmu_bdd_ite_gc,
            purge: None,
            on_return: Some(return_cached_bdd),
            should_flush: None,
        });
        cache.callbacks[CACHE_TYPE_TWO] = Some(CacheCallbacks {
            arity: CacheArity::Three,
            should_collect: bdd_two_gc,
            purge: None,
            on_return: Some(return_cached_bdd),
            should_flush: Some(bdd_two_flush),
        });
        cache.callbacks[CACHE_TYPE_ONEDATA] = Some(CacheCallbacks {
            arity: CacheArity::Two,
            should_collect: cmu_bdd_one_data_gc,
            purge: None,
            on_return: None,
            should_flush: None,
        });
        cache.callbacks[CACHE_TYPE_TWODATA] = Some(CacheCallbacks {
            arity: CacheArity::Three,
            should_collect: bdd_two_data_gc,
            purge: None,
            on_return: None,
            should_flush: None,
        });
        cache
    }

    pub fn size_index(&self) -> usize {
        self.size_index
    }

    pub fn size(&self) -> usize {
        self.bins.len()
    }

    pub fn cache_ratio(&self) -> usize {
        self.cache_ratio
    }

    pub fn cache_level(&self) -> usize {
        self.cache_level
    }

    pub fn stats(&self) -> CacheStats {
        self.stats
    }

    pub fn entries_for_test(&self) -> Vec<CacheEntry> {
        self.bins
            .iter()
            .flat_map(|bin| bin.entries.iter().flatten().copied())
            .collect()
    }

    pub fn set_cache_level(&mut self, cache_level: usize) {
        self.cache_level = cache_level;
    }

    pub fn rehash(&mut self, grow: bool) -> Result<(), CacheError> {
        let new_size_index = if grow {
            self.size_index.checked_add(1)
        } else {
            self.size_index.checked_sub(1)
        }
        .ok_or(CacheError::InvalidSizeIndex(self.size_index))?;

        if new_size_index >= TABLE_SIZES.len() {
            return Err(CacheError::InvalidSizeIndex(new_size_index));
        }

        let old_bins = std::mem::replace(
            &mut self.bins,
            vec![CacheBin::default(); TABLE_SIZES[new_size_index]],
        );
        self.size_index = new_size_index;

        for lru_index in (0..=1).rev() {
            for bin in &old_bins {
                if let Some(entry) = bin.entries[lru_index] {
                    let callbacks = self.callbacks_for(entry.tag)?;
                    let hash = reduce_hash(callbacks.arity.rehash(&entry), self.size());
                    self.purge_lru_at(hash);
                    self.bins[hash].entries[0] = Some(entry);
                }
            }
        }

        Ok(())
    }

    pub fn insert31(
        &mut self,
        tag: CacheTag,
        data1: CacheWord,
        data2: CacheWord,
        data3: CacheWord,
        result: CacheWord,
    ) -> Result<(), CacheError> {
        let hash = reduce_hash(hash3(data1, data2, data3), self.size());
        let entry = self.get_entry(hash, tag)?;
        entry.slots = [data1, data2, data3, result];
        self.stats.inserts += 1;
        Ok(())
    }

    pub fn lookup31(
        &mut self,
        tag: CacheTag,
        data1: CacheWord,
        data2: CacheWord,
        data3: CacheWord,
    ) -> Result<Option<CacheWord>, CacheError> {
        self.stats.lookups += 1;
        let hash = reduce_hash(hash3(data1, data2, data3), self.size());
        let Some(entry) = self.lookup(hash, tag, |entry| {
            entry.slots[0] == data1 && entry.slots[1] == data2 && entry.slots[2] == data3
        })?
        else {
            return Ok(None);
        };
        Ok(Some(entry.slots[3]))
    }

    pub fn insert22(
        &mut self,
        tag: CacheTag,
        data1: CacheWord,
        data2: CacheWord,
        result1: CacheWord,
        result2: CacheWord,
    ) -> Result<(), CacheError> {
        let hash = reduce_hash(hash2(data1, data2), self.size());
        let entry = self.get_entry(hash, tag)?;
        entry.slots = [data1, data2, result1, result2];
        self.stats.inserts += 1;
        Ok(())
    }

    pub fn lookup22(
        &mut self,
        tag: CacheTag,
        data1: CacheWord,
        data2: CacheWord,
    ) -> Result<Option<(CacheWord, CacheWord)>, CacheError> {
        self.stats.lookups += 1;
        let hash = reduce_hash(hash2(data1, data2), self.size());
        let Some(entry) = self.lookup(hash, tag, |entry| {
            entry.slots[0] == data1 && entry.slots[1] == data2
        })?
        else {
            return Ok(None);
        };
        Ok(Some((entry.slots[2], entry.slots[3])))
    }

    pub fn insert13(
        &mut self,
        tag: CacheTag,
        data1: CacheWord,
        result1: CacheWord,
        result2: CacheWord,
        result3: CacheWord,
    ) -> Result<(), CacheError> {
        let hash = reduce_hash(hash1(data1), self.size());
        let entry = self.get_entry(hash, tag)?;
        entry.slots = [data1, result1, result2, result3];
        self.stats.inserts += 1;
        Ok(())
    }

    pub fn lookup13(
        &mut self,
        tag: CacheTag,
        data1: CacheWord,
    ) -> Result<Option<(CacheWord, CacheWord, CacheWord)>, CacheError> {
        self.stats.lookups += 1;
        let hash = reduce_hash(hash1(data1), self.size());
        let Some(entry) = self.lookup(hash, tag, |entry| entry.slots[0] == data1)? else {
            return Ok(None);
        };
        Ok(Some((entry.slots[1], entry.slots[2], entry.slots[3])))
    }

    pub fn purge_cache(&mut self) {
        for bin_index in 0..self.bins.len() {
            for entry_index in 0..=1 {
                let Some(entry) = self.bins[bin_index].entries[entry_index] else {
                    break;
                };

                let should_collect = self.callbacks[entry.tag]
                    .map(|callbacks| (callbacks.should_collect)(&entry))
                    .unwrap_or(true);

                if should_collect {
                    self.purge_entry_at(bin_index, entry_index);
                } else if entry_index == 1 && self.bins[bin_index].entries[0].is_none() {
                    self.bins[bin_index].entries[0] = self.bins[bin_index].entries[1].take();
                }
            }
        }
    }

    pub fn flush_where(&mut self, mut flush: impl FnMut(&CacheEntry) -> bool) {
        for bin_index in 0..self.bins.len() {
            for entry_index in 0..=1 {
                let Some(entry) = self.bins[bin_index].entries[entry_index] else {
                    break;
                };

                if flush(&entry) {
                    self.purge_entry_at(bin_index, entry_index);
                } else if entry_index == 1 && self.bins[bin_index].entries[0].is_none() {
                    self.bins[bin_index].entries[0] = self.bins[bin_index].entries[1].take();
                }
            }
        }
    }

    pub fn flush_with_tag_callback(
        &mut self,
        tag: CacheTag,
        closure: CacheWord,
    ) -> Result<(), CacheError> {
        let flush = self
            .callbacks_for(tag)?
            .should_flush
            .ok_or(CacheError::InvalidTag(tag))?;
        self.flush_where(|entry| entry.tag == tag && flush(entry, closure));
        Ok(())
    }

    pub fn flush_all(&mut self) {
        for bin_index in 0..self.bins.len() {
            for entry_index in 0..=1 {
                if self.bins[bin_index].entries[entry_index].is_some() {
                    self.purge_entry_at(bin_index, entry_index);
                }
            }
        }
    }

    pub fn register_cache_functions(
        &mut self,
        callbacks: CacheCallbacks,
    ) -> Result<CacheTag, CacheError> {
        for tag in CACHE_TYPE_USER1..CACHE_TYPE_USER1 + USER_ENTRY_TYPES {
            if self.callbacks[tag].is_none() {
                self.callbacks[tag] = Some(callbacks);
                return Ok(tag);
            }
        }

        Err(CacheError::NoUserTagsAvailable)
    }

    pub fn free_cache_tag(&mut self, tag: CacheTag) -> Result<(), CacheError> {
        if tag < CACHE_TYPE_USER1
            || tag >= CACHE_TYPE_USER1 + USER_ENTRY_TYPES
            || self.callbacks[tag].is_none()
        {
            return Err(CacheError::UnallocatedUserTag(tag));
        }

        self.flush_where(|entry| entry.tag == tag);
        self.callbacks[tag] = None;
        Ok(())
    }

    fn callbacks_for(&self, tag: CacheTag) -> Result<CacheCallbacks, CacheError> {
        self.callbacks
            .get(tag)
            .copied()
            .flatten()
            .ok_or(CacheError::InvalidTag(tag))
    }

    fn get_entry(&mut self, hash: usize, tag: CacheTag) -> Result<&mut CacheEntry, CacheError> {
        self.callbacks_for(tag)?;

        if self.bins[hash].entries[0].is_some() && self.bins[hash].entries[1].is_some() {
            self.purge_entry_at(hash, 1);
            self.stats.collisions += 1;

            if self.cache_level == 0 {
                self.bins[hash].entries[1] = self.bins[hash].entries[0];
                self.bins[hash].entries[0] = Some(CacheEntry { tag, slots: [0; 4] });
                return Ok(self.bins[hash].entries[0]
                    .as_mut()
                    .expect("new entry exists"));
            }

            self.bins[hash].entries[1] = Some(CacheEntry { tag, slots: [0; 4] });
            return Ok(self.bins[hash].entries[1]
                .as_mut()
                .expect("new entry exists"));
        }

        self.stats.entries += 1;
        let entry_index = usize::from(self.bins[hash].entries[0].is_some());
        self.bins[hash].entries[entry_index] = Some(CacheEntry { tag, slots: [0; 4] });
        Ok(self.bins[hash].entries[entry_index]
            .as_mut()
            .expect("new entry exists"))
    }

    fn lookup(
        &mut self,
        hash: usize,
        tag: CacheTag,
        matches: impl Fn(&CacheEntry) -> bool,
    ) -> Result<Option<CacheEntry>, CacheError> {
        self.callbacks_for(tag)?;

        let matched_index = if self.bins[hash].entries[0]
            .is_some_and(|entry| entry.tag == tag && matches(&entry))
        {
            Some(0)
        } else if self.bins[hash].entries[1]
            .is_some_and(|entry| entry.tag == tag && matches(&entry))
        {
            Some(1)
        } else {
            None
        };

        let Some(matched_index) = matched_index else {
            return Ok(None);
        };

        if matched_index == 1 {
            self.bins[hash].entries.swap(0, 1);
        }

        let callbacks = self.callbacks_for(tag)?;
        if let Some(on_return) = callbacks.on_return {
            if let Some(entry) = self.bins[hash].entries[0].as_mut() {
                on_return(entry);
            }
        }

        self.stats.hits += 1;
        Ok(self.bins[hash].entries[0])
    }

    fn purge_lru_at(&mut self, hash: usize) {
        if self.bins[hash].entries[1].is_some() {
            self.purge_entry_at(hash, 1);
        }
        self.bins[hash].entries[1] = self.bins[hash].entries[0];
        self.bins[hash].entries[0] = None;
    }

    fn purge_entry_at(&mut self, hash: usize, entry_index: usize) {
        if let Some(mut entry) = self.bins[hash].entries[entry_index].take() {
            if let Some(purge) = self.callbacks[entry.tag].and_then(|callbacks| callbacks.purge) {
                purge(&mut entry);
            }
            self.stats.entries = self.stats.entries.saturating_sub(1);
        }
    }
}

impl Default for OperationCache {
    fn default() -> Self {
        Self::new()
    }
}

pub fn cache_arity_from_count(args: usize) -> Result<CacheArity, CacheError> {
    match args {
        1 => Ok(CacheArity::One),
        2 => Ok(CacheArity::Two),
        3 => Ok(CacheArity::Three),
        _ => Err(CacheError::InvalidArity(args)),
    }
}

fn hash1(data1: CacheWord) -> CacheWord {
    data1
}

fn hash2(data1: CacheWord, data2: CacheWord) -> CacheWord {
    data1.wrapping_add(data2.wrapping_shl(1))
}

fn hash3(data1: CacheWord, data2: CacheWord, data3: CacheWord) -> CacheWord {
    data1
        .wrapping_add(data2.wrapping_shl(1))
        .wrapping_add(data3.wrapping_shl(2))
}

fn reduce_hash(hash: CacheWord, size: usize) -> usize {
    hash.rem_euclid(size as CacheWord) as usize
}

fn is_used_bdd_word(word: CacheWord) -> bool {
    word != 0
}

fn return_cached_bdd(_entry: &mut CacheEntry) {}

fn cmu_bdd_ite_gc(entry: &CacheEntry) -> bool {
    entry.slots.iter().any(|slot| !is_used_bdd_word(*slot))
}

fn bdd_two_gc(entry: &CacheEntry) -> bool {
    entry.slots[1..4]
        .iter()
        .any(|slot| !is_used_bdd_word(*slot))
}

fn bdd_two_data_gc(entry: &CacheEntry) -> bool {
    entry.slots[1..3]
        .iter()
        .any(|slot| !is_used_bdd_word(*slot))
}

fn cmu_bdd_one_data_gc(entry: &CacheEntry) -> bool {
    !is_used_bdd_word(entry.slots[1])
}

fn bdd_two_flush(entry: &CacheEntry, id_to_nuke: CacheWord) -> bool {
    entry.slots[0] == OP_RELPROD + id_to_nuke
        || entry.slots[0] == OP_QNT + id_to_nuke
        || entry.slots[0] == OP_SUBST + id_to_nuke
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static PURGED: AtomicUsize = AtomicUsize::new(0);
    static COLLISION_PURGED: AtomicUsize = AtomicUsize::new(0);
    static RETURNED: AtomicUsize = AtomicUsize::new(0);

    fn never_collect(_entry: &CacheEntry) -> bool {
        false
    }

    fn collect_when_first_slot_is_zero(entry: &CacheEntry) -> bool {
        entry.slots[0] == 0
    }

    fn count_purge(_entry: &mut CacheEntry) {
        PURGED.fetch_add(1, Ordering::SeqCst);
    }

    fn count_collision_purge(_entry: &mut CacheEntry) {
        COLLISION_PURGED.fetch_add(1, Ordering::SeqCst);
    }

    fn count_return(_entry: &mut CacheEntry) {
        RETURNED.fetch_add(1, Ordering::SeqCst);
    }

    fn flush_matching_closure(entry: &CacheEntry, closure: CacheWord) -> bool {
        entry.slots[0] == closure
    }

    fn user_callbacks(arity: CacheArity) -> CacheCallbacks {
        CacheCallbacks {
            arity,
            should_collect: never_collect,
            purge: Some(count_purge),
            on_return: Some(count_return),
            should_flush: Some(flush_matching_closure),
        }
    }

    #[test]
    fn cache31_round_trips_and_updates_stats() {
        let mut cache = OperationCache::new();

        cache.insert31(CACHE_TYPE_ITE, 1, 2, 3, 99).unwrap();

        assert_eq!(cache.lookup31(CACHE_TYPE_ITE, 1, 2, 3).unwrap(), Some(99));
        assert_eq!(cache.lookup31(CACHE_TYPE_ITE, 1, 2, 4).unwrap(), None);
        assert_eq!(
            cache.stats(),
            CacheStats {
                entries: 1,
                lookups: 2,
                hits: 1,
                inserts: 1,
                collisions: 0,
            }
        );
    }

    #[test]
    fn cache22_and_cache13_round_trip_multiple_results() {
        let mut cache = OperationCache::new();

        cache.insert22(CACHE_TYPE_ONEDATA, 4, 5, 11, 12).unwrap();
        cache
            .insert13(CACHE_TYPE_USER1, 7, 21, 22, 23)
            .expect_err("unregistered user tag must fail");
        let tag = cache
            .register_cache_functions(user_callbacks(CacheArity::One))
            .unwrap();
        cache.insert13(tag, 7, 21, 22, 23).unwrap();

        assert_eq!(
            cache.lookup22(CACHE_TYPE_ONEDATA, 4, 5).unwrap(),
            Some((11, 12))
        );
        assert_eq!(cache.lookup13(tag, 7).unwrap(), Some((21, 22, 23)));
    }

    #[test]
    fn second_slot_hit_is_promoted_to_mru() {
        let mut cache = OperationCache::new();
        cache.size_index = 0;
        cache.bins = vec![CacheBin::default(); 1];

        cache.insert31(CACHE_TYPE_ITE, 1, 1, 1, 10).unwrap();
        cache.insert31(CACHE_TYPE_ITE, 2, 2, 2, 20).unwrap();

        assert_eq!(cache.bins[0].entries[0].unwrap().slots[3], 10);
        assert_eq!(cache.bins[0].entries[1].unwrap().slots[3], 20);
        assert_eq!(cache.lookup31(CACHE_TYPE_ITE, 2, 2, 2).unwrap(), Some(20));
        assert_eq!(cache.bins[0].entries[0].unwrap().slots[3], 20);
        assert_eq!(cache.bins[0].entries[1].unwrap().slots[3], 10);
    }

    #[test]
    fn collision_purges_lru_and_demotes_mru_at_default_level() {
        COLLISION_PURGED.store(0, Ordering::SeqCst);
        let mut cache = OperationCache::new();
        cache.size_index = 0;
        cache.bins = vec![CacheBin::default(); 1];
        let tag = cache
            .register_cache_functions(CacheCallbacks {
                arity: CacheArity::Three,
                should_collect: never_collect,
                purge: Some(count_collision_purge),
                on_return: None,
                should_flush: None,
            })
            .unwrap();

        cache.insert31(tag, 1, 1, 1, 10).unwrap();
        cache.insert31(tag, 2, 2, 2, 20).unwrap();
        cache.insert31(tag, 3, 3, 3, 30).unwrap();

        assert_eq!(COLLISION_PURGED.load(Ordering::SeqCst), 1);
        assert_eq!(cache.stats().collisions, 1);
        assert_eq!(cache.bins[0].entries[0].unwrap().slots[3], 30);
        assert_eq!(cache.bins[0].entries[1].unwrap().slots[3], 10);
    }

    #[test]
    fn purge_cache_removes_entries_selected_by_gc_callback() {
        let mut cache = OperationCache::new();
        let tag = cache
            .register_cache_functions(CacheCallbacks {
                arity: CacheArity::One,
                should_collect: collect_when_first_slot_is_zero,
                purge: None,
                on_return: None,
                should_flush: None,
            })
            .unwrap();

        cache.insert13(tag, 0, 1, 2, 3).unwrap();
        cache.insert13(tag, 9, 1, 2, 3).unwrap();
        cache.purge_cache();

        assert_eq!(cache.lookup13(tag, 0).unwrap(), None);
        assert_eq!(cache.lookup13(tag, 9).unwrap(), Some((1, 2, 3)));
    }

    #[test]
    fn flush_with_tag_callback_and_free_tag_remove_user_entries() {
        let mut cache = OperationCache::new();
        let tag = cache
            .register_cache_functions(user_callbacks(CacheArity::One))
            .unwrap();

        cache.insert13(tag, 10, 1, 2, 3).unwrap();
        cache.insert13(tag, 11, 4, 5, 6).unwrap();
        cache.flush_with_tag_callback(tag, 10).unwrap();

        assert_eq!(cache.lookup13(tag, 10).unwrap(), None);
        assert_eq!(cache.lookup13(tag, 11).unwrap(), Some((4, 5, 6)));

        cache.free_cache_tag(tag).unwrap();
        assert_eq!(cache.lookup13(tag, 11), Err(CacheError::InvalidTag(tag)));
    }

    #[test]
    fn rehash_preserves_reachable_entries() {
        let mut cache = OperationCache::new();

        cache.insert31(CACHE_TYPE_ITE, 1, 2, 3, 4).unwrap();
        cache.insert31(CACHE_TYPE_TWO, 5, 6, 7, 8).unwrap();
        cache.rehash(true).unwrap();

        assert_eq!(cache.lookup31(CACHE_TYPE_ITE, 1, 2, 3).unwrap(), Some(4));
        assert_eq!(cache.lookup31(CACHE_TYPE_TWO, 5, 6, 7).unwrap(), Some(8));
        assert_eq!(cache.size_index(), INITIAL_SIZE_INDEX + 1);
    }

    #[test]
    fn built_in_two_flush_matches_legacy_operation_ids() {
        let mut cache = OperationCache::new();

        cache
            .insert31(CACHE_TYPE_TWO, OP_RELPROD + 3, 2, 3, 4)
            .unwrap();
        cache
            .insert31(CACHE_TYPE_TWO, OP_RELPROD + 4, 2, 3, 5)
            .unwrap();
        cache.flush_with_tag_callback(CACHE_TYPE_TWO, 3).unwrap();

        assert_eq!(
            cache
                .lookup31(CACHE_TYPE_TWO, OP_RELPROD + 3, 2, 3)
                .unwrap(),
            None
        );
        assert_eq!(
            cache
                .lookup31(CACHE_TYPE_TWO, OP_RELPROD + 4, 2, 3)
                .unwrap(),
            Some(5)
        );
    }
}
