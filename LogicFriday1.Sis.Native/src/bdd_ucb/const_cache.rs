//! Native Rust model of the SIS UCB BDD ITE constant cache.
//!
//! The legacy cache is a direct-mapped table: each hash bucket stores at most
//! one `(f, g, h)` key and insertions overwrite an existing bucket entry. This
//! port preserves that behavior in owned Rust data while exposing resize
//! outcomes and counters explicitly.

use std::error::Error;
use std::fmt;
use std::mem;

pub const CONST_CACHE_INITIAL_SIZE: usize = 113;
pub const POINTER_BYTES: usize = mem::size_of::<usize>();

const HASH_PRIMES: [usize; 28] = [
    3, 11, 23, 59, 113, 251, 503, 1019, 2039, 4091, 8179, 16369, 32749, 65521, 131063, 262139,
    524269, 1048571, 2097143, 4194287, 8388593, 16777199, 33554393, 67108859, 134217689, 268435399,
    536870879, 1073741789,
];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IteKey {
    pub f: BddNodeId,
    pub g: BddNodeId,
    pub h: BddNodeId,
}

impl IteKey {
    pub fn new(f: BddNodeId, g: BddNodeId, h: BddNodeId) -> Self {
        Self { f, g, h }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstantStatus {
    Unknown,
    ConstantZero,
    ConstantOne,
    Nonconstant,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ConstCacheStats {
    pub hits: usize,
    pub misses: usize,
    pub collisions: usize,
    pub inserts: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryLimit {
    pub used_bytes: usize,
    pub limit_bytes: Option<usize>,
}

impl MemoryLimit {
    pub fn unlimited(used_bytes: usize) -> Self {
        Self {
            used_bytes,
            limit_bytes: None,
        }
    }

    pub fn with_limit(used_bytes: usize, limit_bytes: usize) -> Self {
        Self {
            used_bytes,
            limit_bytes: Some(limit_bytes),
        }
    }

    pub fn will_exceed(self, allocation_bytes: usize) -> bool {
        self.limit_bytes
            .is_some_and(|limit| self.used_bytes.saturating_add(allocation_bytes) >= limit)
    }
}

impl Default for MemoryLimit {
    fn default() -> Self {
        Self::unlimited(0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConstCacheConfig {
    pub enabled: bool,
    pub resize_at_percent: usize,
    pub max_size: usize,
}

impl Default for ConstCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            resize_at_percent: 85,
            max_size: 1_073_741_789,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConstCacheEntry {
    pub key: IteKey,
    pub data: ConstantStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstCache {
    buckets: Vec<Option<ConstCacheEntry>>,
    config: ConstCacheConfig,
    stats: ConstCacheStats,
    memory: MemoryLimit,
}

impl ConstCache {
    pub fn new(
        nbuckets: usize,
        config: ConstCacheConfig,
        memory: MemoryLimit,
    ) -> Result<Self, ConstCacheError> {
        if nbuckets == 0 {
            return Err(ConstCacheError::ZeroBuckets);
        }

        Ok(Self {
            buckets: vec![None; nbuckets],
            config,
            stats: ConstCacheStats::default(),
            memory,
        })
    }

    pub fn enabled(nbuckets: usize) -> Result<Self, ConstCacheError> {
        Self::new(
            nbuckets,
            ConstCacheConfig::default(),
            MemoryLimit::unlimited(nbuckets.saturating_mul(POINTER_BYTES)),
        )
    }

    pub fn disabled(nbuckets: usize) -> Result<Self, ConstCacheError> {
        Self::new(
            nbuckets,
            ConstCacheConfig {
                enabled: false,
                ..ConstCacheConfig::default()
            },
            MemoryLimit::unlimited(nbuckets.saturating_mul(POINTER_BYTES)),
        )
    }

    pub fn nbuckets(&self) -> usize {
        self.buckets.len()
    }

    pub fn nentries(&self) -> usize {
        self.buckets.iter().filter(|entry| entry.is_some()).count()
    }

    pub fn stats(&self) -> ConstCacheStats {
        self.stats
    }

    pub fn memory(&self) -> MemoryLimit {
        self.memory
    }

    pub fn bucket(&self, bucket: usize) -> Option<ConstCacheEntry> {
        self.buckets[bucket]
    }

    pub fn lookup(&mut self, key: IteKey) -> Option<ConstantStatus> {
        if !self.config.enabled {
            return None;
        }

        let position = bdd_const_hash(key, self.nbuckets());
        let Some(entry) = self.buckets[position] else {
            self.stats.misses += 1;
            return None;
        };

        if entry.data == ConstantStatus::Unknown || entry.key != key {
            self.stats.misses += 1;
            return None;
        }

        self.stats.hits += 1;
        Some(entry.data)
    }

    pub fn insert(
        &mut self,
        key: IteKey,
        data: ConstantStatus,
    ) -> Result<ConstCacheInsertOutcome, ConstCacheError> {
        if !self.config.enabled {
            return Ok(ConstCacheInsertOutcome::SkippedDisabled);
        }

        let position = bdd_const_hash(key, self.nbuckets());
        let replaced_existing = self.buckets[position].is_some();
        if replaced_existing {
            self.stats.collisions += 1;
        }

        self.buckets[position] = Some(ConstCacheEntry { key, data });
        self.stats.inserts += 1;

        let resize = if self.load_percent() > self.config.resize_at_percent {
            self.resize_consttable()?
        } else {
            ConstCacheResizeOutcome::NotNeeded
        };

        Ok(ConstCacheInsertOutcome::Inserted {
            replaced_existing,
            resize,
        })
    }

    fn load_percent(&self) -> usize {
        self.nentries() * 100 / self.nbuckets()
    }

    fn resize_consttable(&mut self) -> Result<ConstCacheResizeOutcome, ConstCacheError> {
        if self.nbuckets() > self.config.max_size {
            return Ok(ConstCacheResizeOutcome::SkippedMaxSize {
                current_buckets: self.nbuckets(),
            });
        }

        let next_prime = get_next_hash_prime(self.nbuckets())?;
        let allocation_bytes = next_prime
            .checked_mul(POINTER_BYTES)
            .ok_or(ConstCacheError::AllocationSizeOverflow)?;

        if self.memory.will_exceed(allocation_bytes) {
            return Ok(ConstCacheResizeOutcome::SkippedMemoryLimit {
                requested_buckets: next_prime,
                allocation_bytes,
            });
        }

        let old_bucket_bytes = self.nbuckets().saturating_mul(POINTER_BYTES);
        let old_buckets = mem::replace(&mut self.buckets, vec![None; next_prime]);
        let mut dropped_entries = 0;

        for entry in old_buckets.into_iter().flatten() {
            let position = bdd_const_hash(entry.key, next_prime);
            if self.buckets[position].is_some() {
                dropped_entries += 1;
            } else {
                self.buckets[position] = Some(entry);
            }
        }

        self.memory.used_bytes = self
            .memory
            .used_bytes
            .saturating_add(allocation_bytes)
            .saturating_sub(old_bucket_bytes);

        Ok(ConstCacheResizeOutcome::Resized {
            buckets: next_prime,
            dropped_entries,
        })
    }
}

impl Default for ConstCache {
    fn default() -> Self {
        Self::enabled(CONST_CACHE_INITIAL_SIZE).expect("default constant cache is valid")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstCacheInsertOutcome {
    Inserted {
        replaced_existing: bool,
        resize: ConstCacheResizeOutcome,
    },
    SkippedDisabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstCacheResizeOutcome {
    NotNeeded,
    Resized {
        buckets: usize,
        dropped_entries: usize,
    },
    SkippedMaxSize {
        current_buckets: usize,
    },
    SkippedMemoryLimit {
        requested_buckets: usize,
        allocation_bytes: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstCacheError {
    ZeroBuckets,
    UnknownHashPrime(usize),
    LargestHashPrime(usize),
    AllocationSizeOverflow,
}

impl fmt::Display for ConstCacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroBuckets => write!(f, "constant cache must contain at least one bucket"),
            Self::UnknownHashPrime(size) => {
                write!(f, "cache size {size} is not a legacy BDD prime")
            }
            Self::LargestHashPrime(size) => {
                write!(
                    f,
                    "cache size {size} is already the largest legacy BDD prime"
                )
            }
            Self::AllocationSizeOverflow => write!(f, "constant cache allocation size overflowed"),
        }
    }
}

impl Error for ConstCacheError {}

pub fn bdd_const_hash(key: IteKey, nbuckets: usize) -> usize {
    assert!(
        nbuckets > 0,
        "constant cache must contain at least one bucket"
    );

    key.f
        .0
        .wrapping_shl(5)
        .wrapping_add(key.g.0.wrapping_shl(7))
        .wrapping_add(key.h.0.wrapping_shl(11))
        % nbuckets
}

pub fn get_next_hash_prime(current_size: usize) -> Result<usize, ConstCacheError> {
    for pair in HASH_PRIMES.windows(2) {
        if current_size == pair[0] {
            return Ok(pair[1]);
        }
    }

    if current_size == HASH_PRIMES[HASH_PRIMES.len() - 1] {
        Err(ConstCacheError::LargestHashPrime(current_size))
    } else {
        Err(ConstCacheError::UnknownHashPrime(current_size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(f: usize, g: usize, h: usize) -> IteKey {
        IteKey::new(BddNodeId(f), BddNodeId(g), BddNodeId(h))
    }

    #[test]
    fn hash_matches_legacy_generic_hash_shape() {
        let value = bdd_const_hash(key(3, 5, 7), 113);

        assert_eq!(
            value,
            ((3usize << 5) + (5usize << 7) + (7usize << 11)) % 113
        );
    }

    #[test]
    fn disabled_cache_ignores_insert_and_lookup_without_stats() {
        let mut cache = ConstCache::disabled(3).unwrap();

        assert_eq!(
            cache.insert(key(1, 2, 3), ConstantStatus::ConstantOne),
            Ok(ConstCacheInsertOutcome::SkippedDisabled)
        );
        assert_eq!(cache.lookup(key(1, 2, 3)), None);
        assert_eq!(cache.nentries(), 0);
        assert_eq!(cache.stats(), ConstCacheStats::default());
    }

    #[test]
    fn lookup_hits_exact_non_unknown_entry() {
        let mut cache = ConstCache::enabled(113).unwrap();
        let cache_key = key(1, 2, 3);

        let outcome = cache
            .insert(cache_key, ConstantStatus::ConstantZero)
            .unwrap();

        assert_eq!(
            outcome,
            ConstCacheInsertOutcome::Inserted {
                replaced_existing: false,
                resize: ConstCacheResizeOutcome::NotNeeded,
            }
        );
        assert_eq!(cache.lookup(cache_key), Some(ConstantStatus::ConstantZero));
        assert_eq!(
            cache.stats(),
            ConstCacheStats {
                hits: 1,
                misses: 0,
                collisions: 0,
                inserts: 1,
            }
        );
    }

    #[test]
    fn collision_reuses_bucket_and_counts_collision_without_new_entry() {
        let mut cache = ConstCache::enabled(3).unwrap();
        let first = key(0, 0, 0);
        let second = key(3, 0, 0);

        cache.insert(first, ConstantStatus::ConstantZero).unwrap();
        cache.insert(second, ConstantStatus::ConstantOne).unwrap();

        assert_eq!(cache.nentries(), 1);
        assert_eq!(cache.stats().collisions, 1);
        assert_eq!(cache.lookup(first), None);
        assert_eq!(cache.lookup(second), Some(ConstantStatus::ConstantOne));
        assert_eq!(cache.stats().misses, 1);
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn unknown_status_entry_is_treated_as_a_miss() {
        let mut cache = ConstCache::enabled(113).unwrap();
        let cache_key = key(4, 5, 6);

        cache.insert(cache_key, ConstantStatus::Unknown).unwrap();

        assert_eq!(cache.lookup(cache_key), None);
        assert_eq!(cache.stats().misses, 1);
        assert_eq!(cache.stats().hits, 0);
    }

    #[test]
    fn insert_resizes_when_load_percent_exceeds_threshold() {
        let mut cache = ConstCache::new(
            3,
            ConstCacheConfig {
                enabled: true,
                resize_at_percent: 60,
                max_size: 1073741789,
            },
            MemoryLimit::unlimited(3 * POINTER_BYTES),
        )
        .unwrap();

        cache
            .insert(key(0, 0, 0), ConstantStatus::ConstantZero)
            .unwrap();
        let outcome = cache
            .insert(key(1, 0, 0), ConstantStatus::ConstantOne)
            .unwrap();

        assert_eq!(
            outcome,
            ConstCacheInsertOutcome::Inserted {
                replaced_existing: false,
                resize: ConstCacheResizeOutcome::Resized {
                    buckets: 11,
                    dropped_entries: 0,
                },
            }
        );
        assert_eq!(cache.nbuckets(), 11);
        assert_eq!(cache.nentries(), 2);
        assert_eq!(
            cache.lookup(key(0, 0, 0)),
            Some(ConstantStatus::ConstantZero)
        );
        assert_eq!(
            cache.lookup(key(1, 0, 0)),
            Some(ConstantStatus::ConstantOne)
        );
    }

    #[test]
    fn resize_drops_entries_that_collide_in_new_table() {
        let mut cache = ConstCache::new(
            3,
            ConstCacheConfig {
                enabled: true,
                resize_at_percent: 60,
                max_size: 1073741789,
            },
            MemoryLimit::unlimited(3 * POINTER_BYTES),
        )
        .unwrap();

        cache
            .insert(key(0, 0, 0), ConstantStatus::ConstantZero)
            .unwrap();
        let outcome = cache
            .insert(key(11, 0, 0), ConstantStatus::ConstantOne)
            .unwrap();

        assert_eq!(
            outcome,
            ConstCacheInsertOutcome::Inserted {
                replaced_existing: false,
                resize: ConstCacheResizeOutcome::Resized {
                    buckets: 11,
                    dropped_entries: 1,
                },
            }
        );
        assert_eq!(cache.nentries(), 1);
        assert_eq!(
            cache.lookup(key(0, 0, 0)),
            Some(ConstantStatus::ConstantZero)
        );
        assert_eq!(cache.lookup(key(11, 0, 0)), None);
    }

    #[test]
    fn memory_limit_skip_leaves_table_size_unchanged() {
        let requested_bytes = 11 * POINTER_BYTES;
        let mut cache = ConstCache::new(
            3,
            ConstCacheConfig {
                enabled: true,
                resize_at_percent: 60,
                max_size: 1073741789,
            },
            MemoryLimit::with_limit(10, 10 + requested_bytes),
        )
        .unwrap();

        cache
            .insert(key(0, 0, 0), ConstantStatus::ConstantZero)
            .unwrap();
        let outcome = cache
            .insert(key(1, 0, 0), ConstantStatus::ConstantOne)
            .unwrap();

        assert_eq!(
            outcome,
            ConstCacheInsertOutcome::Inserted {
                replaced_existing: false,
                resize: ConstCacheResizeOutcome::SkippedMemoryLimit {
                    requested_buckets: 11,
                    allocation_bytes: requested_bytes,
                },
            }
        );
        assert_eq!(cache.nbuckets(), 3);
        assert_eq!(cache.nentries(), 2);
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_tokens_are_present() {
        let source = include_str!("const_cache.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }
}
