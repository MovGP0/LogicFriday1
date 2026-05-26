//! Native Rust shutdown model for the SIS UCB BDD `bdd_end.c` unit.
//!
//! The C implementation accepts a null manager for compatibility, releases the
//! manager-owned heap structures in declaration order, optionally closes debug
//! resources, and deliberately leaves application-specific data for `bdd_quit`.
//! This port represents that behavior by consuming owned Rust state, returning
//! teardown accounting, and handing the caller-owned application payload back.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddManager<T = ()> {
    pub heap: BddHeap,
    pub debug: BddDebugState,
    pub application_data: T,
}

impl<T> BddManager<T> {
    pub fn new(heap: BddHeap, application_data: T) -> Self {
        Self {
            heap,
            debug: BddDebugState::default(),
            application_data,
        }
    }

    pub fn with_debug(mut self, debug: BddDebugState) -> Self {
        self.debug = debug;
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddHeap {
    pub hashtable_buckets: Vec<Option<BddNodeId>>,
    pub cache: BddCache,
    pub external_refs: Vec<BddBlock>,
    pub halfspaces: [BddHalfspace; 2],
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddCache {
    pub ite_table: BddOwnedTable<BddHashCacheEntry>,
    pub const_table: BddOwnedTable<BddConstCacheEntry>,
    pub adhoc: BddAdhocCache,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddOwnedTable<T> {
    pub buckets: Vec<Option<T>>,
}

impl<T> BddOwnedTable<T> {
    pub fn new(buckets: Vec<Option<T>>) -> Self {
        Self { buckets }
    }
}

impl<T> Default for BddOwnedTable<T> {
    fn default() -> Self {
        Self {
            buckets: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddAdhocCache {
    pub initialized: bool,
    pub entries: Vec<BddAdhocEntry>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddHalfspace {
    pub inuse: Vec<NodeBlock>,
    pub free: Vec<NodeBlock>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddHashCacheEntry {
    pub f: BddNodeId,
    pub g: BddNodeId,
    pub h: BddNodeId,
    pub data: Option<BddNodeId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddConstStatus {
    Unknown,
    ConstantZero,
    ConstantOne,
    Nonconstant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddConstCacheEntry {
    pub f: BddNodeId,
    pub g: BddNodeId,
    pub h: BddNodeId,
    pub data: BddConstStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddAdhocEntry {
    pub f: Option<BddNodeId>,
    pub g: Option<BddNodeId>,
    pub value: i32,
    pub data: Option<BddNodeId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddBlock {
    pub slots: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeBlock {
    pub used: usize,
    pub capacity: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddDebugState {
    pub flight_recorder_open: bool,
    pub lifespan_trace_open: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddEndOptions {
    pub emit_final_manager_stats: bool,
    pub close_flight_recorder: bool,
    pub close_lifespan_trace: bool,
    pub dump_node_ages: bool,
    pub dump_external_pointers: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddEndReport<T = ()> {
    pub released: BddReleasedResources,
    pub debug_events: Vec<BddDebugEvent>,
    pub application_data: T,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddReleasedResources {
    pub hashtable_buckets: usize,
    pub ite_cache_buckets: usize,
    pub ite_cache_entries: usize,
    pub const_cache_buckets: usize,
    pub const_cache_entries: usize,
    pub adhoc_cache_entries: usize,
    pub external_ref_blocks: usize,
    pub external_ref_slots: usize,
    pub inuse_node_blocks: usize,
    pub free_node_blocks: usize,
    pub node_capacity: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddDebugEvent {
    FinalManagerStatsDumped,
    FlightRecorderClosed,
    LifespanTraceClosed,
    NodeAgesDumped,
    ExternalPointersDumped,
}

pub fn bdd_end<T>(manager: Option<BddManager<T>>) -> Option<BddEndReport<T>> {
    bdd_end_with_options(manager, BddEndOptions::default())
}

pub fn bdd_end_with_options<T>(
    manager: Option<BddManager<T>>,
    options: BddEndOptions,
) -> Option<BddEndReport<T>> {
    let mut manager = manager?;
    let mut debug_events = Vec::new();

    if options.emit_final_manager_stats {
        debug_events.push(BddDebugEvent::FinalManagerStatsDumped);
    }

    if options.close_flight_recorder && manager.debug.flight_recorder_open {
        manager.debug.flight_recorder_open = false;
        debug_events.push(BddDebugEvent::FlightRecorderClosed);
    }

    if options.close_lifespan_trace && manager.debug.lifespan_trace_open {
        manager.debug.lifespan_trace_open = false;
        debug_events.push(BddDebugEvent::LifespanTraceClosed);
    }

    if options.dump_node_ages {
        debug_events.push(BddDebugEvent::NodeAgesDumped);
    }

    if options.dump_external_pointers {
        debug_events.push(BddDebugEvent::ExternalPointersDumped);
    }

    let released = release_heap(&mut manager.heap);

    Some(BddEndReport {
        released,
        debug_events,
        application_data: manager.application_data,
    })
}

fn release_heap(heap: &mut BddHeap) -> BddReleasedResources {
    let hashtable_buckets = heap.hashtable_buckets.len();

    let ite_cache_buckets = heap.cache.ite_table.buckets.len();
    let ite_cache_entries = heap
        .cache
        .ite_table
        .buckets
        .iter()
        .filter(|entry| entry.is_some())
        .count();

    let const_cache_buckets = heap.cache.const_table.buckets.len();
    let const_cache_entries = heap
        .cache
        .const_table
        .buckets
        .iter()
        .filter(|entry| entry.is_some())
        .count();

    let adhoc_cache_entries = if heap.cache.adhoc.initialized {
        let entries = heap.cache.adhoc.entries.len();
        heap.cache.adhoc.entries.clear();
        heap.cache.adhoc.initialized = false;
        entries
    } else {
        0
    };

    let external_ref_blocks = heap.external_refs.len();
    let external_ref_slots = heap.external_refs.iter().map(|block| block.slots).sum();

    let inuse_node_blocks = heap
        .halfspaces
        .iter()
        .map(|halfspace| halfspace.inuse.len())
        .sum();
    let free_node_blocks = heap
        .halfspaces
        .iter()
        .map(|halfspace| halfspace.free.len())
        .sum();
    let node_capacity = heap
        .halfspaces
        .iter()
        .flat_map(|halfspace| halfspace.inuse.iter().chain(halfspace.free.iter()))
        .map(|block| block.capacity)
        .sum();

    heap.hashtable_buckets.clear();
    heap.cache.ite_table.buckets.clear();
    heap.cache.const_table.buckets.clear();
    heap.external_refs.clear();
    for halfspace in &mut heap.halfspaces {
        halfspace.inuse.clear();
        halfspace.free.clear();
    }

    BddReleasedResources {
        hashtable_buckets,
        ite_cache_buckets,
        ite_cache_entries,
        const_cache_buckets,
        const_cache_entries,
        adhoc_cache_entries,
        external_ref_blocks,
        external_ref_slots,
        inuse_node_blocks,
        free_node_blocks,
        node_capacity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: usize) -> BddNodeId {
        BddNodeId(id)
    }

    fn hash_entry(id: usize) -> BddHashCacheEntry {
        BddHashCacheEntry {
            f: node(id),
            g: node(id + 1),
            h: node(id + 2),
            data: Some(node(id + 3)),
        }
    }

    fn const_entry(id: usize) -> BddConstCacheEntry {
        BddConstCacheEntry {
            f: node(id),
            g: node(id + 1),
            h: node(id + 2),
            data: BddConstStatus::ConstantOne,
        }
    }

    fn populated_manager() -> BddManager<&'static str> {
        BddManager::new(
            BddHeap {
                hashtable_buckets: vec![Some(node(1)), None, Some(node(2))],
                cache: BddCache {
                    ite_table: BddOwnedTable::new(vec![
                        Some(hash_entry(1)),
                        None,
                        Some(hash_entry(10)),
                    ]),
                    const_table: BddOwnedTable::new(vec![Some(const_entry(1)), None]),
                    adhoc: BddAdhocCache {
                        initialized: true,
                        entries: vec![
                            BddAdhocEntry {
                                f: Some(node(1)),
                                g: None,
                                value: 7,
                                data: Some(node(8)),
                            },
                            BddAdhocEntry {
                                f: None,
                                g: Some(node(2)),
                                value: 9,
                                data: None,
                            },
                        ],
                    },
                },
                external_refs: vec![BddBlock { slots: 32 }, BddBlock { slots: 16 }],
                halfspaces: [
                    BddHalfspace {
                        inuse: vec![
                            NodeBlock {
                                used: 4,
                                capacity: 8,
                            },
                            NodeBlock {
                                used: 1,
                                capacity: 8,
                            },
                        ],
                        free: vec![NodeBlock {
                            used: 0,
                            capacity: 8,
                        }],
                    },
                    BddHalfspace {
                        inuse: vec![NodeBlock {
                            used: 2,
                            capacity: 8,
                        }],
                        free: vec![
                            NodeBlock {
                                used: 0,
                                capacity: 8,
                            },
                            NodeBlock {
                                used: 0,
                                capacity: 8,
                            },
                        ],
                    },
                ],
            },
            "keep-for-bdd-quit",
        )
    }

    #[test]
    fn none_manager_is_accepted_for_legacy_compatibility() {
        let report = bdd_end::<()>(None);

        assert_eq!(report, None);
    }

    #[test]
    fn end_reports_manager_owned_heap_resources() {
        let report = bdd_end(Some(populated_manager())).unwrap();

        assert_eq!(
            report.released,
            BddReleasedResources {
                hashtable_buckets: 3,
                ite_cache_buckets: 3,
                ite_cache_entries: 2,
                const_cache_buckets: 2,
                const_cache_entries: 1,
                adhoc_cache_entries: 2,
                external_ref_blocks: 2,
                external_ref_slots: 48,
                inuse_node_blocks: 3,
                free_node_blocks: 3,
                node_capacity: 48,
            }
        );
    }

    #[test]
    fn application_payload_is_returned_untouched() {
        let report = bdd_end(Some(populated_manager())).unwrap();

        assert_eq!(report.application_data, "keep-for-bdd-quit");
    }

    #[test]
    fn uninitialized_adhoc_cache_is_not_counted_as_uninitialized_work() {
        let mut manager = populated_manager();
        manager.heap.cache.adhoc.initialized = false;

        let report = bdd_end(Some(manager)).unwrap();

        assert_eq!(report.released.adhoc_cache_entries, 0);
    }

    #[test]
    fn debug_options_emit_and_close_only_present_resources() {
        let manager = populated_manager().with_debug(BddDebugState {
            flight_recorder_open: true,
            lifespan_trace_open: false,
        });

        let report = bdd_end_with_options(
            Some(manager),
            BddEndOptions {
                emit_final_manager_stats: true,
                close_flight_recorder: true,
                close_lifespan_trace: true,
                dump_node_ages: true,
                dump_external_pointers: true,
            },
        )
        .unwrap();

        assert_eq!(
            report.debug_events,
            vec![
                BddDebugEvent::FinalManagerStatsDumped,
                BddDebugEvent::FlightRecorderClosed,
                BddDebugEvent::NodeAgesDumped,
                BddDebugEvent::ExternalPointersDumped,
            ]
        );
    }

    #[test]
    fn empty_manager_reports_zero_released_resources() {
        let report = bdd_end(Some(BddManager::new(BddHeap::default(), ()))).unwrap();

        assert_eq!(report.released, BddReleasedResources::default());
        assert!(report.debug_events.is_empty());
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_tokens_are_present() {
        let source = include_str!("bdd_end.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern", " ")));
        assert!(!source.contains(concat!("pub ", "unsafe ", "extern", " ")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }
}
