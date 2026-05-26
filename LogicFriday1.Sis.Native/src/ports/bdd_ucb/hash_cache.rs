#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HashCacheKey {
    pub f: BddNodeId,
    pub g: BddNodeId,
    pub h: BddNodeId,
}

impl HashCacheKey {
    pub const fn new(f: BddNodeId, g: BddNodeId, h: BddNodeId) -> Self {
        Self { f, g, h }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HashCacheEntry {
    pub key: HashCacheKey,
    pub data: Option<BddNodeId>,
}

impl HashCacheEntry {
    pub const fn new(key: HashCacheKey, data: BddNodeId) -> Self {
        Self {
            key,
            data: Some(data),
        }
    }

    pub const fn invalid(key: HashCacheKey) -> Self {
        Self { key, data: None }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HashCacheStats {
    pub inserts: usize,
    pub hits: usize,
    pub misses: usize,
    pub collisions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HashCache {
    on: bool,
    resize_at: usize,
    max_size: usize,
    buckets: Vec<Option<HashCacheEntry>>,
    stats: HashCacheStats,
}

impl HashCache {
    pub fn new(initial_size: usize, resize_at: usize, max_size: usize) -> Self {
        assert!(initial_size > 0, "hash cache requires at least one bucket");

        Self {
            on: true,
            resize_at,
            max_size,
            buckets: vec![None; initial_size],
            stats: HashCacheStats::default(),
        }
    }

    pub fn with_default_sizes() -> Self {
        Self::new(113, 80, usize::MAX)
    }

    pub const fn is_on(&self) -> bool {
        self.on
    }

    pub fn set_on(&mut self, on: bool) {
        self.on = on;
    }

    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    pub fn entry_count(&self) -> usize {
        self.buckets
            .iter()
            .filter(|bucket| bucket.is_some())
            .count()
    }

    pub const fn stats(&self) -> HashCacheStats {
        self.stats
    }

    pub fn insert(&mut self, f: BddNodeId, g: BddNodeId, h: BddNodeId, data: BddNodeId) {
        if !self.on {
            return;
        }

        let key = HashCacheKey::new(f, g, h);
        let pos = self.hash_key(key);
        if self.buckets[pos].is_some() {
            self.stats.collisions += 1;
        }

        self.buckets[pos] = Some(HashCacheEntry::new(key, data));
        self.stats.inserts += 1;

        if self.load_percent() > self.resize_at {
            self.resize_itetable();
        }
    }

    pub fn lookup(&mut self, f: BddNodeId, g: BddNodeId, h: BddNodeId) -> Option<BddNodeId> {
        if !self.on {
            return None;
        }

        let key = HashCacheKey::new(f, g, h);
        let pos = self.hash_key(key);
        let Some(entry) = self.buckets[pos] else {
            self.stats.misses += 1;
            return None;
        };

        if entry.key != key {
            self.stats.misses += 1;
            return None;
        }

        let Some(data) = entry.data else {
            self.stats.misses += 1;
            return None;
        };

        self.stats.hits += 1;
        Some(data)
    }

    pub fn invalidate_slot_for(&mut self, f: BddNodeId, g: BddNodeId, h: BddNodeId) {
        let key = HashCacheKey::new(f, g, h);
        let pos = self.hash_key(key);
        if let Some(entry) = self.buckets[pos].as_mut() {
            if entry.key == key {
                entry.data = None;
            }
        }
    }

    pub fn hash_position(&self, key: HashCacheKey) -> usize {
        self.hash_key(key)
    }

    fn load_percent(&self) -> usize {
        (self.entry_count() * 100) / self.buckets.len()
    }

    fn resize_itetable(&mut self) {
        if self.buckets.len() > self.max_size {
            return;
        }

        let Some(next_size) = next_hash_prime(self.buckets.len()) else {
            return;
        };

        let old_buckets = std::mem::replace(&mut self.buckets, vec![None; next_size]);

        for old_entry in old_buckets.into_iter().flatten() {
            let pos = self.hash_key(old_entry.key);
            if self.buckets[pos].is_none() {
                self.buckets[pos] = Some(old_entry);
            }
        }
    }

    fn hash_key(&self, key: HashCacheKey) -> usize {
        generic_hash(key.f.0, key.g.0, key.h.0, self.buckets.len())
    }
}

pub fn generic_hash(a: usize, b: usize, c: usize, nbuckets: usize) -> usize {
    assert!(nbuckets > 0, "hash requires at least one bucket");

    a.wrapping_shl(5)
        .wrapping_add(b.wrapping_shl(7))
        .wrapping_add(c.wrapping_shl(11))
        % nbuckets
}

pub fn next_hash_prime(current_size: usize) -> Option<usize> {
    HASH_PRIMES
        .windows(2)
        .find_map(|pair| (pair[0] == current_size).then_some(pair[1]))
}

pub const HASH_PRIMES: [usize; 28] = [
    3, 11, 23, 59, 113, 251, 503, 1019, 2039, 4091, 8179, 16369, 32749, 65521, 131063, 262139,
    524269, 1048571, 2097143, 4194287, 8388593, 16777199, 33554393, 67108859, 134217689, 268435399,
    536870879, 1073741789,
];

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: usize) -> BddNodeId {
        BddNodeId(id)
    }

    #[test]
    fn lookup_returns_inserted_ite_result() {
        let mut cache = HashCache::new(113, 80, usize::MAX);

        cache.insert(node(1), node(2), node(3), node(42));

        assert_eq!(cache.lookup(node(1), node(2), node(3)), Some(node(42)));
        assert_eq!(cache.stats().hits, 1);
        assert_eq!(cache.stats().misses, 0);
    }

    #[test]
    fn cache_off_ignores_inserts_and_lookups() {
        let mut cache = HashCache::new(113, 80, usize::MAX);
        cache.set_on(false);

        cache.insert(node(1), node(2), node(3), node(42));

        assert_eq!(cache.lookup(node(1), node(2), node(3)), None);
        assert_eq!(cache.entry_count(), 0);
        assert_eq!(cache.stats(), HashCacheStats::default());
    }

    #[test]
    fn occupied_slot_is_reused_as_collision() {
        let mut cache = HashCache::new(3, 100, usize::MAX);
        let first = HashCacheKey::new(node(0), node(0), node(0));
        let second = HashCacheKey::new(node(3), node(0), node(0));
        assert_eq!(cache.hash_position(first), cache.hash_position(second));

        cache.insert(first.f, first.g, first.h, node(10));
        cache.insert(second.f, second.g, second.h, node(11));

        assert_eq!(cache.entry_count(), 1);
        assert_eq!(cache.stats().collisions, 1);
        assert_eq!(cache.lookup(first.f, first.g, first.h), None);
        assert_eq!(cache.lookup(second.f, second.g, second.h), Some(node(11)));
    }

    #[test]
    fn lookup_miss_counts_empty_mismatched_and_invalid_slots() {
        let mut cache = HashCache::new(113, 100, usize::MAX);
        cache.insert(node(1), node(2), node(3), node(4));

        assert_eq!(cache.lookup(node(5), node(6), node(7)), None);
        assert_eq!(cache.lookup(node(1), node(2), node(4)), None);
        cache.invalidate_slot_for(node(1), node(2), node(3));
        assert_eq!(cache.lookup(node(1), node(2), node(3)), None);
        assert_eq!(cache.stats().misses, 3);
    }

    #[test]
    fn insert_resizes_after_load_threshold() {
        let mut cache = HashCache::new(3, 65, usize::MAX);

        cache.insert(node(1), node(0), node(0), node(10));
        assert_eq!(cache.bucket_count(), 3);

        cache.insert(node(2), node(0), node(0), node(20));

        assert_eq!(cache.bucket_count(), 11);
        assert_eq!(cache.lookup(node(1), node(0), node(0)), Some(node(10)));
        assert_eq!(cache.lookup(node(2), node(0), node(0)), Some(node(20)));
    }

    #[test]
    fn resize_keeps_only_first_entry_for_new_slot_collision() {
        let mut cache = HashCache::new(3, 65, usize::MAX);
        let first = HashCacheKey::new(node(0), node(0), node(0));
        let second = HashCacheKey::new(node(11), node(0), node(0));
        assert_ne!(cache.hash_position(first), cache.hash_position(second));

        cache.insert(first.f, first.g, first.h, node(10));
        cache.insert(second.f, second.g, second.h, node(20));
        assert_eq!(cache.bucket_count(), 11);

        assert_eq!(cache.lookup(first.f, first.g, first.h), Some(node(10)));
        assert_eq!(cache.lookup(second.f, second.g, second.h), None);
        assert_eq!(cache.entry_count(), 1);
    }

    #[test]
    fn max_size_guard_prevents_resize_once_current_table_is_larger() {
        let mut cache = HashCache::new(3, 0, 1);

        cache.insert(node(1), node(0), node(0), node(10));

        assert_eq!(cache.bucket_count(), 3);
    }

    #[test]
    fn next_prime_matches_legacy_table() {
        assert_eq!(next_hash_prime(3), Some(11));
        assert_eq!(next_hash_prime(113), Some(251));
        assert_eq!(next_hash_prime(1073741789), None);
    }
}
