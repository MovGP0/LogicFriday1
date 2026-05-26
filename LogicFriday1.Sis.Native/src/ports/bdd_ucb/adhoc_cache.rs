use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AdhocCacheKey {
    pub f: Option<BddNodeId>,
    pub g: Option<BddNodeId>,
    pub v: isize,
}

impl AdhocCacheKey {
    pub const fn new(f: Option<BddNodeId>, g: Option<BddNodeId>, v: isize) -> Self {
        Self { f, g, v }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdhocCacheConfig {
    pub enabled: bool,
    pub max_size: usize,
}

impl Default for AdhocCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size: usize::MAX,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AdhocCacheStats {
    pub inserts: usize,
    pub hits: usize,
    pub misses: usize,
    pub resets: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdhocCache {
    table: Option<HashMap<AdhocCacheKey, BddNodeId>>,
    config: AdhocCacheConfig,
    stats: AdhocCacheStats,
}

impl AdhocCache {
    pub fn new(config: AdhocCacheConfig) -> Self {
        Self {
            table: Some(HashMap::new()),
            config,
            stats: AdhocCacheStats::default(),
        }
    }

    pub fn enabled(max_size: usize) -> Self {
        Self::new(AdhocCacheConfig {
            enabled: true,
            max_size,
        })
    }

    pub fn disabled(max_size: usize) -> Self {
        Self::new(AdhocCacheConfig {
            enabled: false,
            max_size,
        })
    }

    pub fn init(&mut self) {
        self.table = Some(HashMap::new());
    }

    pub fn uninit(&mut self) {
        self.table = None;
    }

    pub fn clear(&mut self) {
        if let Some(table) = self.table.as_mut() {
            table.clear();
        }
    }

    pub const fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    pub const fn max_size(&self) -> usize {
        self.config.max_size
    }

    pub fn set_max_size(&mut self, max_size: usize) {
        self.config.max_size = max_size;
    }

    pub const fn stats(&self) -> AdhocCacheStats {
        self.stats
    }

    pub fn entry_count(&self) -> usize {
        self.table.as_ref().map_or(0, HashMap::len)
    }

    pub fn is_initialized(&self) -> bool {
        self.table.is_some()
    }

    pub fn contains_key(&self, key: AdhocCacheKey) -> bool {
        self.table
            .as_ref()
            .is_some_and(|table| table.contains_key(&key))
    }

    pub fn insert(
        &mut self,
        key: AdhocCacheKey,
        value: BddNodeId,
    ) -> Result<AdhocInsertOutcome, AdhocCacheError> {
        if !self.config.enabled {
            return Ok(AdhocInsertOutcome::SkippedDisabled);
        }

        let mut reset_before_insert = false;
        if self.entry_count() > self.config.max_size {
            self.init();
            self.stats.resets += 1;
            reset_before_insert = true;
        }

        let table = self.table.as_mut().ok_or(AdhocCacheError::Uninitialized)?;
        if table.contains_key(&key) {
            return Err(AdhocCacheError::DuplicateKey(key));
        }

        table.insert(key, value);
        self.stats.inserts += 1;

        Ok(AdhocInsertOutcome::Inserted {
            reset_before_insert,
        })
    }

    pub fn lookup(&mut self, key: AdhocCacheKey) -> Option<BddNodeId> {
        if !self.config.enabled {
            return None;
        }

        let Some(table) = self.table.as_ref() else {
            self.stats.misses += 1;
            return None;
        };

        if let Some(value) = table.get(&key) {
            self.stats.hits += 1;
            Some(*value)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    pub fn hash_position(key: AdhocCacheKey, modulus: usize) -> usize {
        assert!(modulus > 0, "ad-hoc cache hash modulus must be non-zero");

        bdd_generic_hash(
            key.f.map_or(0, |node| node.0),
            key.g.map_or(0, |node| node.0),
            key.v as usize,
            modulus,
        )
    }
}

impl Default for AdhocCache {
    fn default() -> Self {
        Self::new(AdhocCacheConfig::default())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdhocInsertOutcome {
    Inserted { reset_before_insert: bool },
    SkippedDisabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdhocCacheError {
    Uninitialized,
    DuplicateKey(AdhocCacheKey),
}

impl fmt::Display for AdhocCacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uninitialized => write!(f, "ad-hoc cache is not initialized"),
            Self::DuplicateKey(_) => write!(f, "ad-hoc cache key already exists"),
        }
    }
}

impl Error for AdhocCacheError {}

pub fn bdd_generic_hash(a: usize, b: usize, c: usize, modulus: usize) -> usize {
    assert!(modulus > 0, "generic hash modulus must be non-zero");

    a.wrapping_shl(5)
        .wrapping_add(b.wrapping_shl(7))
        .wrapping_add(c.wrapping_shl(11))
        % modulus
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: usize) -> BddNodeId {
        BddNodeId(id)
    }

    fn key(f: Option<usize>, g: Option<usize>, v: isize) -> AdhocCacheKey {
        AdhocCacheKey::new(f.map(node), g.map(node), v)
    }

    #[test]
    fn lookup_returns_inserted_entry_and_updates_stats() {
        let mut cache = AdhocCache::enabled(10);
        let cache_key = key(Some(1), Some(2), 3);

        assert_eq!(
            cache.insert(cache_key, node(42)),
            Ok(AdhocInsertOutcome::Inserted {
                reset_before_insert: false,
            })
        );

        assert_eq!(cache.lookup(cache_key), Some(node(42)));
        assert_eq!(
            cache.stats(),
            AdhocCacheStats {
                inserts: 1,
                hits: 1,
                misses: 0,
                resets: 0,
            }
        );
    }

    #[test]
    fn disabled_cache_ignores_insert_and_lookup_without_stats() {
        let mut cache = AdhocCache::disabled(10);
        let cache_key = key(Some(1), None, 0);

        assert_eq!(
            cache.insert(cache_key, node(4)),
            Ok(AdhocInsertOutcome::SkippedDisabled)
        );
        assert_eq!(cache.lookup(cache_key), None);
        assert_eq!(cache.entry_count(), 0);
        assert_eq!(cache.stats(), AdhocCacheStats::default());
    }

    #[test]
    fn lookup_distinguishes_optional_nodes_and_integer_discriminator() {
        let mut cache = AdhocCache::enabled(10);

        cache.insert(key(Some(1), None, 0), node(10)).unwrap();
        cache.insert(key(Some(1), Some(0), 0), node(11)).unwrap();
        cache.insert(key(Some(1), None, -1), node(12)).unwrap();

        assert_eq!(cache.lookup(key(Some(1), None, 0)), Some(node(10)));
        assert_eq!(cache.lookup(key(Some(1), Some(0), 0)), Some(node(11)));
        assert_eq!(cache.lookup(key(Some(1), None, -1)), Some(node(12)));
        assert_eq!(cache.lookup(key(Some(1), None, 1)), None);
        assert_eq!(cache.stats().hits, 3);
        assert_eq!(cache.stats().misses, 1);
    }

    #[test]
    fn duplicate_insert_is_reported_as_invalid() {
        let mut cache = AdhocCache::enabled(10);
        let cache_key = key(Some(1), Some(2), 3);

        cache.insert(cache_key, node(42)).unwrap();

        assert_eq!(
            cache.insert(cache_key, node(43)),
            Err(AdhocCacheError::DuplicateKey(cache_key))
        );
        assert_eq!(cache.lookup(cache_key), Some(node(42)));
        assert_eq!(cache.stats().inserts, 1);
    }

    #[test]
    fn insert_resets_existing_table_only_after_count_exceeds_max_size() {
        let mut cache = AdhocCache::enabled(1);

        cache.insert(key(Some(1), None, 0), node(10)).unwrap();
        cache.insert(key(Some(2), None, 0), node(20)).unwrap();
        assert_eq!(cache.entry_count(), 2);
        assert_eq!(cache.stats().resets, 0);

        assert_eq!(
            cache.insert(key(Some(3), None, 0), node(30)),
            Ok(AdhocInsertOutcome::Inserted {
                reset_before_insert: true,
            })
        );

        assert_eq!(cache.entry_count(), 1);
        assert_eq!(cache.lookup(key(Some(1), None, 0)), None);
        assert_eq!(cache.lookup(key(Some(3), None, 0)), Some(node(30)));
        assert_eq!(cache.stats().resets, 1);
        assert_eq!(cache.stats().inserts, 3);
    }

    #[test]
    fn uninitialized_cache_rejects_insert_and_counts_lookup_miss() {
        let mut cache = AdhocCache::enabled(10);
        let cache_key = key(Some(1), None, 0);

        cache.uninit();

        assert_eq!(
            cache.insert(cache_key, node(9)),
            Err(AdhocCacheError::Uninitialized)
        );
        assert_eq!(cache.lookup(cache_key), None);
        assert_eq!(cache.stats().misses, 1);

        cache.init();
        cache.insert(cache_key, node(9)).unwrap();
        assert_eq!(cache.lookup(cache_key), Some(node(9)));
    }

    #[test]
    fn hash_matches_legacy_generic_hash_shape() {
        let cache_key = key(Some(3), Some(5), 7);

        assert_eq!(
            AdhocCache::hash_position(cache_key, 113),
            ((3usize << 5) + (5usize << 7) + (7usize << 11)) % 113
        );
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_tokens_are_present() {
        let source = include_str!("adhoc_cache.rs");

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
