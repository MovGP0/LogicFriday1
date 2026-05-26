//! Native Rust startup model for the SIS UCB BDD manager.
//!
//! The legacy startup routine allocated one manager, filled every owned table
//! and counter with defaults, created the constant-one node through the unique
//! table, and allowed a memory daemon callback to be replaced later. This port
//! keeps that state explicit in safe Rust data without preserving per-file C
//! ABI entry points.

use std::error::Error;
use std::fmt;
use std::mem;

pub const BDD_ONE_ID: BddVariableId = BddVariableId(1 << 30);
pub const HASHTABLE_INITIAL_SIZE: usize = 113;
pub const CACHE_INITIAL_SIZE: usize = 113;
pub const HASHTABLE_MAX_CHAIN_LEN: usize = 4;
pub const DEFAULT_ITE_CACHE_ON: bool = true;
pub const DEFAULT_ITE_CACHE_RESIZE_AT: usize = 75;
pub const DEFAULT_ITE_CACHE_MAX_SIZE: usize = 1_000_000;
pub const DEFAULT_ITE_CONST_CACHE_ON: bool = true;
pub const DEFAULT_ITE_CONST_CACHE_RESIZE_AT: usize = 75;
pub const DEFAULT_ITE_CONST_CACHE_MAX_SIZE: usize = 1_000_000;
pub const DEFAULT_ADHOC_CACHE_ON: bool = true;
pub const DEFAULT_ADHOC_CACHE_RESIZE_AT: usize = 0;
pub const DEFAULT_ADHOC_CACHE_MAX_SIZE: usize = 10_000_000;
pub const DEFAULT_GARBAGE_COLLECTOR_ON: bool = true;
pub const DEFAULT_NODE_RATIO: f64 = 2.0;
pub const DEFAULT_INIT_BLOCKS: usize = 10;
pub const POINTER_BYTES: usize = mem::size_of::<usize>();

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddVariableId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddPointer {
    node: BddNodeId,
    complemented: bool,
}

impl BddPointer {
    pub const fn positive(node: BddNodeId) -> Self {
        Self {
            node,
            complemented: false,
        }
    }

    pub const fn complemented(node: BddNodeId) -> Self {
        Self {
            node,
            complemented: true,
        }
    }

    pub const fn node(self) -> BddNodeId {
        self.node
    }

    pub const fn is_complemented(self) -> bool {
        self.complemented
    }

    fn hash_value(self) -> usize {
        (self.node.0 << 1) | usize::from(self.complemented)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddNodeKey {
    pub variable_id: BddVariableId,
    pub then_child: Option<BddPointer>,
    pub else_child: Option<BddPointer>,
}

impl BddNodeKey {
    pub const fn new(
        variable_id: BddVariableId,
        then_child: Option<BddPointer>,
        else_child: Option<BddPointer>,
    ) -> Self {
        Self {
            variable_id,
            then_child,
            else_child,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddNode {
    key: BddNodeKey,
    next: Option<BddNodeId>,
}

impl BddNode {
    pub const fn key(&self) -> BddNodeKey {
        self.key
    }

    pub const fn next(&self) -> Option<BddNodeId> {
        self.next
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CacheInit {
    pub on: bool,
    pub resize_at: usize,
    pub max_size: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GarbageCollectorInit {
    pub on: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryDaemon {
    None,
    Registered(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryInit {
    pub daemon: MemoryDaemon,
    pub limit: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeInit {
    pub ratio: f64,
    pub init_blocks: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BddManagerInit {
    pub ite_cache: CacheInit,
    pub ite_const_cache: CacheInit,
    pub adhoc_cache: CacheInit,
    pub garbage_collector: GarbageCollectorInit,
    pub memory: MemoryInit,
    pub nodes: NodeInit,
}

impl Default for BddManagerInit {
    fn default() -> Self {
        Self {
            ite_cache: CacheInit {
                on: DEFAULT_ITE_CACHE_ON,
                resize_at: DEFAULT_ITE_CACHE_RESIZE_AT,
                max_size: DEFAULT_ITE_CACHE_MAX_SIZE,
            },
            ite_const_cache: CacheInit {
                on: DEFAULT_ITE_CONST_CACHE_ON,
                resize_at: DEFAULT_ITE_CONST_CACHE_RESIZE_AT,
                max_size: DEFAULT_ITE_CONST_CACHE_MAX_SIZE,
            },
            adhoc_cache: CacheInit {
                on: DEFAULT_ADHOC_CACHE_ON,
                resize_at: DEFAULT_ADHOC_CACHE_RESIZE_AT,
                max_size: DEFAULT_ADHOC_CACHE_MAX_SIZE,
            },
            garbage_collector: GarbageCollectorInit {
                on: DEFAULT_GARBAGE_COLLECTOR_ON,
            },
            memory: MemoryInit {
                daemon: MemoryDaemon::None,
                limit: None,
            },
            nodes: NodeInit {
                ratio: DEFAULT_NODE_RATIO,
                init_blocks: DEFAULT_INIT_BLOCKS,
            },
        }
    }
}

pub fn set_manager_init_defaults(init: &mut BddManagerInit) {
    *init = BddManagerInit::default();
}

pub fn manager_init_defaults() -> BddManagerInit {
    BddManagerInit::default()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UniqueTable {
    buckets: Vec<Option<BddNodeId>>,
    nodes: Vec<BddNode>,
    rehash_at_nkeys: usize,
}

impl UniqueTable {
    pub fn new(nbuckets: usize) -> Result<Self, BddStartError> {
        if nbuckets == 0 {
            return Err(BddStartError::ZeroBuckets);
        }

        Ok(Self {
            buckets: vec![None; nbuckets],
            nodes: Vec::new(),
            rehash_at_nkeys: nbuckets * HASHTABLE_MAX_CHAIN_LEN,
        })
    }

    pub fn nbuckets(&self) -> usize {
        self.buckets.len()
    }

    pub fn nkeys(&self) -> usize {
        self.nodes.len()
    }

    pub fn rehash_at_nkeys(&self) -> usize {
        self.rehash_at_nkeys
    }

    pub fn node(&self, node: BddNodeId) -> Option<&BddNode> {
        self.nodes.get(node.0)
    }

    pub fn bucket_chain(&self, bucket: usize) -> Vec<BddNodeId> {
        let mut chain = Vec::new();
        let mut cursor = self.buckets[bucket];

        while let Some(node) = cursor {
            chain.push(node);
            cursor = self.nodes[node.0].next;
        }

        chain
    }

    pub fn find_or_add(&mut self, key: BddNodeKey) -> BddNodeId {
        let bucket = bdd_node_hash(key, self.nbuckets());
        let mut cursor = self.buckets[bucket];

        while let Some(node) = cursor {
            if self.nodes[node.0].key == key {
                return node;
            }

            cursor = self.nodes[node.0].next;
        }

        let node = BddNodeId(self.nodes.len());
        let entry = BddNode {
            key,
            next: self.buckets[bucket],
        };
        self.nodes.push(entry);
        self.buckets[bucket] = Some(node);
        node
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectCache<T> {
    on: bool,
    invalidate_on_gc: bool,
    resize_at: usize,
    max_size: usize,
    buckets: Vec<Option<T>>,
}

impl<T> DirectCache<T> {
    pub fn new(init: CacheInit, nbuckets: usize) -> Result<Self, BddStartError> {
        if nbuckets == 0 {
            return Err(BddStartError::ZeroBuckets);
        }

        Ok(Self {
            on: init.on,
            invalidate_on_gc: false,
            resize_at: init.resize_at,
            max_size: init.max_size,
            buckets: (0..nbuckets).map(|_| None).collect(),
        })
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    pub fn invalidate_on_gc(&self) -> bool {
        self.invalidate_on_gc
    }

    pub fn resize_at(&self) -> usize {
        self.resize_at
    }

    pub fn max_size(&self) -> usize {
        self.max_size
    }

    pub fn nbuckets(&self) -> usize {
        self.buckets.len()
    }

    pub fn nentries(&self) -> usize {
        self.buckets
            .iter()
            .filter(|bucket| bucket.is_some())
            .count()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HashCacheEntry;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConstCacheEntry;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdhocCache {
    on: bool,
    max_size: usize,
    table_allocated: bool,
}

impl AdhocCache {
    fn new(init: CacheInit) -> Self {
        Self {
            on: init.on,
            max_size: init.max_size,
            table_allocated: false,
        }
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    pub fn max_size(&self) -> usize {
        self.max_size
    }

    pub fn table_allocated(&self) -> bool {
        self.table_allocated
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddCaches {
    pub ite: DirectCache<HashCacheEntry>,
    pub constant: DirectCache<ConstCacheEntry>,
    pub adhoc: AdhocCache,
}

impl BddCaches {
    fn new(init: BddManagerInit) -> Result<Self, BddStartError> {
        Ok(Self {
            ite: DirectCache::new(init.ite_cache, CACHE_INITIAL_SIZE)?,
            constant: DirectCache::new(init.ite_const_cache, CACHE_INITIAL_SIZE)?,
            adhoc: AdhocCache::new(init.adhoc_cache),
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub collisions: usize,
    pub inserts: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IteOpReturns {
    pub trivial: usize,
    pub cached: usize,
    pub full: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IteOpStats {
    pub calls: usize,
    pub returns: IteOpReturns,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NodeStats {
    pub used: usize,
    pub unused: usize,
    pub total: usize,
    pub peak: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExternalPointerStats {
    pub used: usize,
    pub total: usize,
    pub unused: usize,
    pub blocks: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GcStats {
    pub times: usize,
    pub nodes_collected: usize,
    pub runtime_millis: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MemoryStats {
    pub first_sbrk: usize,
    pub last_sbrk: usize,
    pub manager: usize,
    pub nodes: usize,
    pub hashtable: usize,
    pub external_pointers: usize,
    pub ite_cache: usize,
    pub ite_const_cache: usize,
    pub adhoc_cache: usize,
    pub total: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BlockStats {
    pub total: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddStats {
    pub hashtable: CacheStats,
    pub ite_table: CacheStats,
    pub const_table: CacheStats,
    pub adhoc: CacheStats,
    pub ite_ops: IteOpStats,
    pub ite_constant_ops: IteOpStats,
    pub adhoc_ops: IteOpStats,
    pub blocks: BlockStats,
    pub nodes: NodeStats,
    pub external_pointers: ExternalPointerStats,
    pub garbage_collection: GcStats,
    pub memory: MemoryStats,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GarbageCollectorState {
    pub on: bool,
    pub halfspace: usize,
    pub node_ratio: f64,
    pub open_generators: usize,
    pub during_start_index: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExternalRefs {
    pub pointer_block_allocated: bool,
    pub pointer_index: usize,
    pub nmap: usize,
    pub free: usize,
    pub map_allocated: bool,
}

impl Default for ExternalRefs {
    fn default() -> Self {
        Self {
            pointer_block_allocated: false,
            pointer_index: 0,
            nmap: 0,
            free: 0,
            map_allocated: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AllocationCursor {
    pub block_allocated: bool,
    pub index: usize,
}

impl Default for AllocationCursor {
    fn default() -> Self {
        Self {
            block_allocated: false,
            index: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BddHeap {
    pub unique_table: UniqueTable,
    pub external_refs: ExternalRefs,
    pub internal_safe_frames: usize,
    pub caches: BddCaches,
    pub init_node_blocks: usize,
    pub pointer: AllocationCursor,
    pub garbage_collection: GarbageCollectorState,
    pub stats: BddStats,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddRoots {
    pub one: BddPointer,
    pub nvariables: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BddManager {
    pub heap: BddHeap,
    pub memory: MemoryInit,
    pub roots: BddRoots,
    pub hooks_cleared: bool,
    pub debug_age: usize,
    pub debug_unique_id: usize,
}

impl BddManager {
    pub fn register_memory_daemon(&mut self, daemon: MemoryDaemon) {
        self.memory.daemon = daemon;
    }

    pub fn one(&self) -> BddPointer {
        self.roots.one
    }

    pub fn nvariables(&self) -> usize {
        self.roots.nvariables
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddStartError {
    ZeroBuckets,
    InvalidNodeRatio,
    ZeroInitialBlocks,
}

impl fmt::Display for BddStartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroBuckets => write!(f, "BDD tables must contain at least one bucket"),
            Self::InvalidNodeRatio => write!(f, "BDD node allocation ratio must be positive"),
            Self::ZeroInitialBlocks => {
                write!(f, "BDD manager requires at least one initial node block")
            }
        }
    }
}

impl Error for BddStartError {}

pub fn start_bdd_manager(nvariables: usize) -> Result<BddManager, BddStartError> {
    start_bdd_manager_with_params(nvariables, manager_init_defaults())
}

pub fn start_bdd_manager_with_params(
    nvariables: usize,
    init: BddManagerInit,
) -> Result<BddManager, BddStartError> {
    validate_init(init)?;

    let mut unique_table = UniqueTable::new(HASHTABLE_INITIAL_SIZE)?;
    let one = unique_table.find_or_add(BddNodeKey::new(BDD_ONE_ID, None, None));
    let external_refs = ExternalRefs::default();
    let stats = BddStats {
        external_pointers: ExternalPointerStats {
            used: 0,
            total: external_refs.nmap,
            unused: external_refs.nmap,
            blocks: 0,
        },
        memory: MemoryStats {
            hashtable: HASHTABLE_INITIAL_SIZE * POINTER_BYTES,
            ite_cache: CACHE_INITIAL_SIZE * POINTER_BYTES,
            ite_const_cache: CACHE_INITIAL_SIZE * POINTER_BYTES,
            total: (HASHTABLE_INITIAL_SIZE + CACHE_INITIAL_SIZE + CACHE_INITIAL_SIZE)
                * POINTER_BYTES,
            ..MemoryStats::default()
        },
        ..BddStats::default()
    };

    Ok(BddManager {
        heap: BddHeap {
            unique_table,
            external_refs,
            internal_safe_frames: 0,
            caches: BddCaches::new(init)?,
            init_node_blocks: init.nodes.init_blocks,
            pointer: AllocationCursor::default(),
            garbage_collection: GarbageCollectorState {
                on: init.garbage_collector.on,
                halfspace: 0,
                node_ratio: init.nodes.ratio,
                open_generators: 0,
                during_start_index: 0,
            },
            stats,
        },
        memory: init.memory,
        roots: BddRoots {
            one: BddPointer::positive(one),
            nvariables,
        },
        hooks_cleared: true,
        debug_age: 0,
        debug_unique_id: 0,
    })
}

fn validate_init(init: BddManagerInit) -> Result<(), BddStartError> {
    if init.nodes.init_blocks == 0 {
        return Err(BddStartError::ZeroInitialBlocks);
    }

    if !init.nodes.ratio.is_finite() || init.nodes.ratio <= 0.0 {
        return Err(BddStartError::InvalidNodeRatio);
    }

    Ok(())
}

pub fn register_memory_daemon(manager: &mut BddManager, daemon: MemoryDaemon) {
    manager.register_memory_daemon(daemon);
}

pub fn bdd_node_hash(key: BddNodeKey, nbuckets: usize) -> usize {
    assert!(nbuckets > 0, "hash table must contain at least one bucket");

    let then_child = key.then_child.map_or(0usize, BddPointer::hash_value);
    let else_child = key.else_child.map_or(0usize, BddPointer::hash_value);

    key.variable_id
        .0
        .wrapping_shl(5)
        .wrapping_add(then_child.wrapping_shl(7))
        .wrapping_add(else_child.wrapping_shl(11))
        % nbuckets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_init_matches_legacy_header_values() {
        let init = manager_init_defaults();

        assert_eq!(
            init.ite_cache,
            CacheInit {
                on: true,
                resize_at: 75,
                max_size: 1_000_000,
            }
        );
        assert_eq!(
            init.ite_const_cache,
            CacheInit {
                on: true,
                resize_at: 75,
                max_size: 1_000_000,
            }
        );
        assert_eq!(
            init.adhoc_cache,
            CacheInit {
                on: true,
                resize_at: 0,
                max_size: 10_000_000,
            }
        );
        assert_eq!(init.garbage_collector.on, true);
        assert_eq!(init.memory.daemon, MemoryDaemon::None);
        assert_eq!(init.memory.limit, None);
        assert_eq!(init.nodes.ratio, 2.0);
        assert_eq!(init.nodes.init_blocks, 10);
    }

    #[test]
    fn set_defaults_replaces_existing_values() {
        let mut init = BddManagerInit {
            ite_cache: CacheInit {
                on: false,
                resize_at: 1,
                max_size: 2,
            },
            ..manager_init_defaults()
        };

        set_manager_init_defaults(&mut init);

        assert_eq!(init, manager_init_defaults());
    }

    #[test]
    fn start_initializes_empty_tables_caches_and_gc_state() {
        let manager = start_bdd_manager(37).unwrap();

        assert_eq!(manager.nvariables(), 37);
        assert_eq!(manager.heap.unique_table.nbuckets(), HASHTABLE_INITIAL_SIZE);
        assert_eq!(manager.heap.unique_table.rehash_at_nkeys(), 452);
        assert_eq!(manager.heap.unique_table.nkeys(), 1);
        assert_eq!(manager.heap.caches.ite.nbuckets(), CACHE_INITIAL_SIZE);
        assert_eq!(manager.heap.caches.ite.nentries(), 0);
        assert_eq!(manager.heap.caches.constant.nbuckets(), CACHE_INITIAL_SIZE);
        assert_eq!(manager.heap.caches.constant.nentries(), 0);
        assert!(manager.heap.caches.adhoc.is_on());
        assert!(!manager.heap.caches.adhoc.table_allocated());
        assert!(manager.heap.garbage_collection.on);
        assert_eq!(manager.heap.garbage_collection.halfspace, 0);
        assert_eq!(manager.heap.garbage_collection.open_generators, 0);
        assert_eq!(manager.heap.garbage_collection.during_start_index, 0);
        assert_eq!(manager.heap.external_refs, ExternalRefs::default());
        assert_eq!(manager.heap.internal_safe_frames, 0);
        assert_eq!(manager.heap.pointer, AllocationCursor::default());
        assert!(manager.hooks_cleared);
    }

    #[test]
    fn start_creates_constant_one_node_in_unique_table() {
        let manager = start_bdd_manager(3).unwrap();
        let one = manager.one();
        let node = manager.heap.unique_table.node(one.node()).unwrap();

        assert!(!one.is_complemented());
        assert_eq!(one.node(), BddNodeId(0));
        assert_eq!(node.key(), BddNodeKey::new(BDD_ONE_ID, None, None));
        let bucket = bdd_node_hash(node.key(), manager.heap.unique_table.nbuckets());
        assert_eq!(
            manager.heap.unique_table.bucket_chain(bucket),
            vec![one.node()]
        );
    }

    #[test]
    fn custom_params_are_copied_into_manager_state() {
        let mut init = manager_init_defaults();
        init.ite_cache.on = false;
        init.ite_cache.resize_at = 60;
        init.ite_cache.max_size = 7;
        init.ite_const_cache.on = false;
        init.adhoc_cache.on = false;
        init.adhoc_cache.max_size = 11;
        init.garbage_collector.on = false;
        init.memory.limit = Some(4096);
        init.nodes.ratio = 3.5;
        init.nodes.init_blocks = 4;

        let manager = start_bdd_manager_with_params(8, init).unwrap();

        assert!(!manager.heap.caches.ite.is_on());
        assert_eq!(manager.heap.caches.ite.resize_at(), 60);
        assert_eq!(manager.heap.caches.ite.max_size(), 7);
        assert!(!manager.heap.caches.constant.is_on());
        assert!(!manager.heap.caches.adhoc.is_on());
        assert_eq!(manager.heap.caches.adhoc.max_size(), 11);
        assert!(!manager.heap.garbage_collection.on);
        assert_eq!(manager.heap.garbage_collection.node_ratio, 3.5);
        assert_eq!(manager.heap.init_node_blocks, 4);
        assert_eq!(manager.memory.limit, Some(4096));
    }

    #[test]
    fn invalid_node_params_are_rejected() {
        let mut init = manager_init_defaults();
        init.nodes.init_blocks = 0;
        assert_eq!(
            start_bdd_manager_with_params(1, init),
            Err(BddStartError::ZeroInitialBlocks)
        );

        init = manager_init_defaults();
        init.nodes.ratio = 0.0;
        assert_eq!(
            start_bdd_manager_with_params(1, init),
            Err(BddStartError::InvalidNodeRatio)
        );

        init.nodes.ratio = f64::NAN;
        assert_eq!(
            start_bdd_manager_with_params(1, init),
            Err(BddStartError::InvalidNodeRatio)
        );
    }

    #[test]
    fn memory_daemon_can_be_registered_after_start() {
        let mut manager = start_bdd_manager(1).unwrap();

        register_memory_daemon(&mut manager, MemoryDaemon::Registered("limit-handler"));

        assert_eq!(
            manager.memory.daemon,
            MemoryDaemon::Registered("limit-handler")
        );
    }

    #[test]
    fn hash_matches_legacy_unique_table_formula() {
        let key = BddNodeKey::new(
            BddVariableId(3),
            Some(BddPointer::positive(BddNodeId(5))),
            Some(BddPointer::complemented(BddNodeId(7))),
        );
        let then_hash = BddPointer::positive(BddNodeId(5)).hash_value();
        let else_hash = BddPointer::complemented(BddNodeId(7)).hash_value();

        assert_eq!(
            bdd_node_hash(key, 113),
            ((3usize << 5) + (then_hash << 7) + (else_hash << 11)) % 113
        );
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_tokens_are_present() {
        let text = include_str!("bdd_start.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern", " fn")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
        assert!(!text.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }
}
