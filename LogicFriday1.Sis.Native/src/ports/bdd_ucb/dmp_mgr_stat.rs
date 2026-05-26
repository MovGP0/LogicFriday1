//! Native Rust manager statistics dump for the UCB BDD package.
//!
//! The original SIS routine collects already-maintained BDD counters, adds
//! manager-owned cache and safe-frame memory accounting, then prints either a
//! parseable statistics stream or a human-readable report. This port keeps the
//! same calculation boundaries with owned Rust data and writer-based output.

use std::io;
use std::io::Write;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub collisions: u64,
    pub inserts: u64,
}

impl CacheStats {
    pub fn lookups(self) -> u64 {
        self.hits + self.misses
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CacheSummary {
    pub hashtable: CacheStats,
    pub itetable: CacheStats,
    pub consttable: CacheStats,
    pub adhoc: CacheStats,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReturnStats {
    pub trivial: u64,
    pub cached: u64,
    pub full: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OperationStats {
    pub calls: u64,
    pub returns: ReturnStats,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NodeBlockStats {
    pub total: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NodeStats {
    pub used: u64,
    pub unused: u64,
    pub total: u64,
    pub peak: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExternalPointerStats {
    pub used: u64,
    pub unused: u64,
    pub total: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GarbageCollectionStats {
    pub times: u64,
    pub nodes_collected: u64,
    pub runtime_millis: i64,
}

impl GarbageCollectionStats {
    pub fn runtime_seconds(self) -> f64 {
        self.runtime_millis as f64 / 1000.0
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MemoryStats {
    pub first_sbrk: i64,
    pub last_sbrk: i64,
    pub manager: u64,
    pub nodes: u64,
    pub hashtable: u64,
    pub ext_ptrs: u64,
    pub adhoc_cache: u64,
    pub total: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddStats {
    pub cache: CacheSummary,
    pub ite_ops: OperationStats,
    pub ite_constant_ops: OperationStats,
    pub adhoc_ops: OperationStats,
    pub blocks: NodeBlockStats,
    pub nodes: NodeStats,
    pub extptrs: ExternalPointerStats,
    pub gc: GarbageCollectionStats,
    pub memory: MemoryStats,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CacheTableLayout {
    pub buckets: u64,
    pub entries: u64,
    pub bucket_pointer_size: u64,
    pub entry_size: u64,
}

impl CacheTableLayout {
    pub const fn new(
        buckets: u64,
        entries: u64,
        bucket_pointer_size: u64,
        entry_size: u64,
    ) -> Self {
        Self {
            buckets,
            entries,
            bucket_pointer_size,
            entry_size,
        }
    }

    pub fn bucket_memory(self) -> u64 {
        self.buckets * self.bucket_pointer_size
    }

    pub fn entry_memory(self) -> u64 {
        self.entries * self.entry_size
    }
}

impl Default for CacheTableLayout {
    fn default() -> Self {
        Self {
            buckets: 0,
            entries: 0,
            bucket_pointer_size: std::mem::size_of::<usize>() as u64,
            entry_size: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ManagerCacheLayout {
    pub itetable: CacheTableLayout,
    pub consttable: CacheTableLayout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SafeFrameLayout {
    pub frames: u64,
    pub nodes: u64,
    pub frame_size: u64,
    pub node_size: u64,
}

impl SafeFrameLayout {
    pub const fn new(frames: u64, nodes: u64, frame_size: u64, node_size: u64) -> Self {
        Self {
            frames,
            nodes,
            frame_size,
            node_size,
        }
    }

    pub fn memory(self) -> u64 {
        self.frames * self.frame_size + self.nodes * self.node_size
    }
}

impl Default for SafeFrameLayout {
    fn default() -> Self {
        Self {
            frames: 0,
            nodes: 0,
            frame_size: std::mem::size_of::<usize>() as u64 * 2,
            node_size: std::mem::size_of::<usize>() as u64 * 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ManagerSnapshot {
    pub stats: BddStats,
    pub cache_layout: ManagerCacheLayout,
    pub safe_frames: SafeFrameLayout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManagerStatsFormat {
    Automated,
    HumanReadable,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DerivedManagerStats {
    pub total_hashtable_queries: u64,
    pub total_itetable_lookups: u64,
    pub total_consttable_lookups: u64,
    pub total_adhoc_lookups: u64,
    pub safe_frame_memory: u64,
    pub overall_memory: u64,
    pub total_sbrk: i64,
    pub total_overhead: i64,
    pub total_overhead_percent: i64,
}

pub fn percentage(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64 * 100.0
    }
}

pub fn derive_manager_stats(snapshot: &ManagerSnapshot) -> DerivedManagerStats {
    let stats = &snapshot.stats;
    let overall_memory = stats.memory.total + snapshot.safe_frames.memory();
    let total_sbrk = stats.memory.last_sbrk - stats.memory.first_sbrk;
    let total_overhead = total_sbrk - overall_memory as i64;
    let total_overhead_percent = if overall_memory == 0 {
        0
    } else {
        total_overhead * 100 / overall_memory as i64
    };

    DerivedManagerStats {
        total_hashtable_queries: stats.cache.hashtable.lookups(),
        total_itetable_lookups: stats.cache.itetable.lookups(),
        total_consttable_lookups: stats.cache.consttable.lookups(),
        total_adhoc_lookups: stats.cache.adhoc.lookups(),
        safe_frame_memory: snapshot.safe_frames.memory(),
        overall_memory,
        total_sbrk,
        total_overhead,
        total_overhead_percent,
    }
}

pub fn format_manager_stats(snapshot: &ManagerSnapshot, format: ManagerStatsFormat) -> String {
    let mut output = Vec::new();
    write_manager_stats(snapshot, format, &mut output)
        .expect("writing to String buffer cannot fail");
    String::from_utf8(output).expect("BDD manager statistics report is ASCII")
}

pub fn write_manager_stats<W>(
    snapshot: &ManagerSnapshot,
    format: ManagerStatsFormat,
    writer: &mut W,
) -> io::Result<()>
where
    W: Write,
{
    match format {
        ManagerStatsFormat::Automated => write_automated_manager_stats(snapshot, writer),
        ManagerStatsFormat::HumanReadable => write_human_readable_manager_stats(snapshot, writer),
    }
}

fn write_automated_manager_stats<W>(snapshot: &ManagerSnapshot, writer: &mut W) -> io::Result<()>
where
    W: Write,
{
    let stats = &snapshot.stats;
    let cache = &snapshot.cache_layout;
    let derived = derive_manager_stats(snapshot);

    write!(
        writer,
        "\
stats: start
stats: bdd_nodeBlock {}
stats: bdd_node used: {}, unused: {}, total: {}, peak: {}
stats: bdd_t used: {}, unused: {}, total: {}
stats: hash table total: {}, hits: {} ({:4.1}%), misses: {} ({:4.1}%)
stats: ITE ops total: {}, trivial: {} ({:4.1}%), cached: {} ({:4.1}%), full: {} ({:4.1}%)
stats: ITE table lookups: {}, misses: {} ({:4.1}%)
stats: ITE table insertions: {}, collisions: {} ({:4.1}%)
stats: ITE table entries: {}, percent of buckets: {:4.1}%
stats: ITE_const ops total: {}, trivial: {} ({:4.1}%), cached: {} ({:4.1}%), full: {} ({:4.1}%)
stats: ITE_const table lookups: {}, misses: {} ({:4.1}%)
stats: ITE_const table insertions: {}, collisions: {} ({:4.1}%)
stats: ITE_const table entries: {}, percent of buckets: {:4.1}%
stats: adhoc ops total: {}, trivial: {} ({:4.1}%), cached: {} ({:4.1}%), full: {} ({:4.1}%)
stats: adhoc table lookups: {}, misses: {} ({:4.1}%)
stats: garbage-collections: {}
stats: nodes collected: {}
stats: gc runtime: {:.2} sec
stats: end

mem: start (all figures in bytes)
mem: manager            = {:9}
mem: bdd_nodes          = {:9}
mem: unique table bckts = {:9}
mem: external ptrs      = {:9}
mem: ITE buckets        = {:9}
mem: ITE entries        = {:9}
mem: consttable buckets = {:9}
mem: consttable entries = {:9}
mem: adhoc table        = {:9}
mem: safe frames        = {:9}
mem: overall            = {:9}
mem: total sbrk         = {:9}
mem: total overhead     = {:9} ({}%)
mem: end
",
        stats.blocks.total,
        stats.nodes.used,
        stats.nodes.unused,
        stats.nodes.total,
        stats.nodes.peak,
        stats.extptrs.used,
        stats.extptrs.unused,
        stats.extptrs.total,
        derived.total_hashtable_queries,
        stats.cache.hashtable.hits,
        percentage(stats.cache.hashtable.hits, derived.total_hashtable_queries),
        stats.cache.hashtable.misses,
        percentage(
            stats.cache.hashtable.misses,
            derived.total_hashtable_queries
        ),
        stats.ite_ops.calls,
        stats.ite_ops.returns.trivial,
        percentage(stats.ite_ops.returns.trivial, stats.ite_ops.calls),
        stats.ite_ops.returns.cached,
        percentage(stats.ite_ops.returns.cached, stats.ite_ops.calls),
        stats.ite_ops.returns.full,
        percentage(stats.ite_ops.returns.full, stats.ite_ops.calls),
        derived.total_itetable_lookups,
        stats.cache.itetable.misses,
        percentage(stats.cache.itetable.misses, derived.total_itetable_lookups),
        stats.cache.itetable.inserts,
        stats.cache.itetable.collisions,
        percentage(
            stats.cache.itetable.collisions,
            stats.cache.itetable.inserts
        ),
        cache.itetable.entries,
        percentage(cache.itetable.entries, cache.itetable.buckets),
        stats.ite_constant_ops.calls,
        stats.ite_constant_ops.returns.trivial,
        percentage(
            stats.ite_constant_ops.returns.trivial,
            stats.ite_constant_ops.calls
        ),
        stats.ite_constant_ops.returns.cached,
        percentage(
            stats.ite_constant_ops.returns.cached,
            stats.ite_constant_ops.calls
        ),
        stats.ite_constant_ops.returns.full,
        percentage(
            stats.ite_constant_ops.returns.full,
            stats.ite_constant_ops.calls
        ),
        derived.total_consttable_lookups,
        stats.cache.consttable.misses,
        percentage(
            stats.cache.consttable.misses,
            derived.total_consttable_lookups
        ),
        stats.cache.consttable.inserts,
        stats.cache.consttable.collisions,
        percentage(
            stats.cache.consttable.collisions,
            stats.cache.consttable.inserts
        ),
        cache.consttable.entries,
        percentage(cache.consttable.entries, cache.consttable.buckets),
        stats.adhoc_ops.calls,
        stats.adhoc_ops.returns.trivial,
        percentage(stats.adhoc_ops.returns.trivial, stats.adhoc_ops.calls),
        stats.adhoc_ops.returns.cached,
        percentage(stats.adhoc_ops.returns.cached, stats.adhoc_ops.calls),
        stats.adhoc_ops.returns.full,
        percentage(stats.adhoc_ops.returns.full, stats.adhoc_ops.calls),
        derived.total_adhoc_lookups,
        stats.cache.adhoc.misses,
        percentage(stats.cache.adhoc.misses, derived.total_adhoc_lookups),
        stats.gc.times,
        stats.gc.nodes_collected,
        stats.gc.runtime_seconds(),
        stats.memory.manager,
        stats.memory.nodes,
        stats.memory.hashtable,
        stats.memory.ext_ptrs,
        cache.itetable.bucket_memory(),
        cache.itetable.entry_memory(),
        cache.consttable.bucket_memory(),
        cache.consttable.entry_memory(),
        stats.memory.adhoc_cache,
        derived.safe_frame_memory,
        derived.overall_memory,
        derived.total_sbrk,
        derived.total_overhead,
        derived.total_overhead_percent
    )
}

fn write_human_readable_manager_stats<W>(
    snapshot: &ManagerSnapshot,
    writer: &mut W,
) -> io::Result<()>
where
    W: Write,
{
    let stats = &snapshot.stats;
    let derived = derive_manager_stats(snapshot);

    write!(
        writer,
        "\
BDD Manager Statistics

Blocks (bdd_nodeBlock): {}

Nodes (bdd_node):
        used   unused    total     peak
    {:8} {:8} {:8} {:8}

Extptr (bdd_t):
        used   unused    total
    {:8} {:8} {:8}

Hashtable:
    hits:   {:8} ({:4.1}%)
    misses: {:8} ({:4.1}%)
    total:  {:8}

Caches:              ITE    ITE_const     adhoc
 Total calls:    {:8}   {:8}   {:8}
    trivial:    {:9.1}% {:9.1}% {:9.1}%
    cached:     {:9.1}% {:9.1}% {:9.1}%
    full:       {:9.1}% {:9.1}% {:9.1}%
 Total lookups:  {:8}   {:8}   {:8}
    misses:     {:9.1}% {:9.1}% {:9.1}%
 Total inserts:  {:8}   {:8}        --
    collisions: {:9.1}% {:9.1}%       --

Garbage Collections:
    collections: {}
    total nodes collected: {}
    total time:  {:.2} sec

Memory Usage (bytes):
  manager:              {:9}
  bdd_nodes:            {:9}
  unique table buckets: {:9}
  external ptrs:        {:9}
  ITE buckets:          {:9}
  ITE entries:          {:9}
  consttable buckets:   {:9}
  consttable entries:   {:9}
  adhoc table:          {:9}
  safe frames:          {:9}
  overall:              {:9}
  total sbrk:           {:9}
  total overhead:       {:9} ({}%)
",
        stats.blocks.total,
        stats.nodes.used,
        stats.nodes.unused,
        stats.nodes.total,
        stats.nodes.peak,
        stats.extptrs.used,
        stats.extptrs.unused,
        stats.extptrs.total,
        stats.cache.hashtable.hits,
        percentage(stats.cache.hashtable.hits, derived.total_hashtable_queries),
        stats.cache.hashtable.misses,
        percentage(
            stats.cache.hashtable.misses,
            derived.total_hashtable_queries
        ),
        derived.total_hashtable_queries,
        stats.ite_ops.calls,
        stats.ite_constant_ops.calls,
        stats.adhoc_ops.calls,
        percentage(stats.ite_ops.returns.trivial, stats.ite_ops.calls),
        percentage(
            stats.ite_constant_ops.returns.trivial,
            stats.ite_constant_ops.calls
        ),
        percentage(stats.adhoc_ops.returns.trivial, stats.adhoc_ops.calls),
        percentage(stats.ite_ops.returns.cached, stats.ite_ops.calls),
        percentage(
            stats.ite_constant_ops.returns.cached,
            stats.ite_constant_ops.calls
        ),
        percentage(stats.adhoc_ops.returns.cached, stats.adhoc_ops.calls),
        percentage(stats.ite_ops.returns.full, stats.ite_ops.calls),
        percentage(
            stats.ite_constant_ops.returns.full,
            stats.ite_constant_ops.calls
        ),
        percentage(stats.adhoc_ops.returns.full, stats.adhoc_ops.calls),
        derived.total_itetable_lookups,
        derived.total_consttable_lookups,
        derived.total_adhoc_lookups,
        percentage(stats.cache.itetable.misses, derived.total_itetable_lookups),
        percentage(
            stats.cache.consttable.misses,
            derived.total_consttable_lookups
        ),
        percentage(stats.cache.adhoc.misses, derived.total_adhoc_lookups),
        stats.cache.itetable.inserts,
        stats.cache.consttable.inserts,
        percentage(
            stats.cache.itetable.collisions,
            stats.cache.itetable.inserts
        ),
        percentage(
            stats.cache.consttable.collisions,
            stats.cache.consttable.inserts
        ),
        stats.gc.times,
        stats.gc.nodes_collected,
        stats.gc.runtime_seconds(),
        stats.memory.manager,
        stats.memory.nodes,
        stats.memory.hashtable,
        stats.memory.ext_ptrs,
        snapshot.cache_layout.itetable.bucket_memory(),
        snapshot.cache_layout.itetable.entry_memory(),
        snapshot.cache_layout.consttable.bucket_memory(),
        snapshot.cache_layout.consttable.entry_memory(),
        stats.memory.adhoc_cache,
        derived.safe_frame_memory,
        derived.overall_memory,
        derived.total_sbrk,
        derived.total_overhead,
        derived.total_overhead_percent
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentage_returns_zero_for_empty_denominator() {
        assert_eq!(percentage(5, 0), 0.0);
        assert_eq!(percentage(1, 4), 25.0);
    }

    #[test]
    fn derived_stats_match_legacy_intermediate_calculations() {
        let snapshot = sample_snapshot();
        let derived = derive_manager_stats(&snapshot);

        assert_eq!(derived.total_hashtable_queries, 40);
        assert_eq!(derived.total_itetable_lookups, 10);
        assert_eq!(derived.total_consttable_lookups, 20);
        assert_eq!(derived.total_adhoc_lookups, 4);
        assert_eq!(derived.safe_frame_memory, 88);
        assert_eq!(derived.overall_memory, 888);
        assert_eq!(derived.total_sbrk, 1200);
        assert_eq!(derived.total_overhead, 312);
        assert_eq!(derived.total_overhead_percent, 35);
    }

    #[test]
    fn automated_formatter_preserves_parseable_legacy_report_shape() {
        let report = format_manager_stats(&sample_snapshot(), ManagerStatsFormat::Automated);

        assert!(report.starts_with("stats: start\n"));
        assert!(report.contains("stats: bdd_nodeBlock 3\n"));
        assert!(
            report.contains("stats: hash table total: 40, hits: 30 (75.0%), misses: 10 (25.0%)\n")
        );
        assert!(report.contains(
            "stats: ITE ops total: 10, trivial: 1 (10.0%), cached: 3 (30.0%), full: 6 (60.0%)\n"
        ));
        assert!(report.contains("stats: ITE table entries: 5, percent of buckets: 50.0%\n"));
        assert!(report.contains("stats: ITE_const table entries: 6, percent of buckets: 50.0%\n"));
        assert!(report.contains("stats: gc runtime: 2.50 sec\n"));
        assert!(report.contains("mem: safe frames        =        88\n"));
        assert!(report.contains("mem: overall            =       888\n"));
        assert!(report.contains("mem: total sbrk         =      1200\n"));
        assert!(report.contains("mem: total overhead     =       312 (35%)\n"));
        assert!(report.ends_with("mem: end\n"));
    }

    #[test]
    fn human_readable_formatter_includes_manager_specific_memory_lines() {
        let report = format_manager_stats(&sample_snapshot(), ManagerStatsFormat::HumanReadable);

        assert!(report.starts_with("BDD Manager Statistics\n\n"));
        assert!(report.contains("  ITE buckets:                 80\n"));
        assert!(report.contains("  ITE entries:                120\n"));
        assert!(report.contains("  consttable buckets:          96\n"));
        assert!(report.contains("  consttable entries:         168\n"));
        assert!(report.contains("  safe frames:                 88\n"));
        assert!(report.contains("  total overhead:             312 (35%)\n"));
    }

    #[test]
    fn zero_snapshot_formats_without_division_errors() {
        let report =
            format_manager_stats(&ManagerSnapshot::default(), ManagerStatsFormat::Automated);

        assert!(
            report.contains("stats: hash table total: 0, hits: 0 ( 0.0%), misses: 0 ( 0.0%)\n")
        );
        assert!(report.contains("stats: ITE table entries: 0, percent of buckets:  0.0%\n"));
        assert!(report.contains("mem: total overhead     =         0 (0%)\n"));
    }

    #[test]
    fn writer_api_propagates_output_errors() {
        struct FailingWriter;

        impl Write for FailingWriter {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("sink failed"))
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let error = write_manager_stats(
            &ManagerSnapshot::default(),
            ManagerStatsFormat::Automated,
            &mut FailingWriter,
        )
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Other);
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_dependency_metadata_are_present() {
        let source = include_str!("dmp_mgr_stat.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }

    fn sample_snapshot() -> ManagerSnapshot {
        ManagerSnapshot {
            stats: BddStats {
                cache: CacheSummary {
                    hashtable: CacheStats {
                        hits: 30,
                        misses: 10,
                        collisions: 0,
                        inserts: 0,
                    },
                    itetable: CacheStats {
                        hits: 6,
                        misses: 4,
                        collisions: 4,
                        inserts: 10,
                    },
                    consttable: CacheStats {
                        hits: 12,
                        misses: 8,
                        collisions: 2,
                        inserts: 8,
                    },
                    adhoc: CacheStats {
                        hits: 3,
                        misses: 1,
                        collisions: 0,
                        inserts: 0,
                    },
                },
                ite_ops: OperationStats {
                    calls: 10,
                    returns: ReturnStats {
                        trivial: 1,
                        cached: 3,
                        full: 6,
                    },
                },
                ite_constant_ops: OperationStats {
                    calls: 20,
                    returns: ReturnStats {
                        trivial: 5,
                        cached: 7,
                        full: 8,
                    },
                },
                adhoc_ops: OperationStats {
                    calls: 25,
                    returns: ReturnStats {
                        trivial: 5,
                        cached: 5,
                        full: 15,
                    },
                },
                blocks: NodeBlockStats { total: 3 },
                nodes: NodeStats {
                    used: 4,
                    unused: 5,
                    total: 9,
                    peak: 12,
                },
                extptrs: ExternalPointerStats {
                    used: 6,
                    unused: 7,
                    total: 13,
                },
                gc: GarbageCollectionStats {
                    times: 2,
                    nodes_collected: 55,
                    runtime_millis: 2500,
                },
                memory: MemoryStats {
                    first_sbrk: 100,
                    last_sbrk: 1300,
                    manager: 100,
                    nodes: 200,
                    hashtable: 300,
                    ext_ptrs: 400,
                    adhoc_cache: 700,
                    total: 800,
                },
            },
            cache_layout: ManagerCacheLayout {
                itetable: CacheTableLayout::new(10, 5, 8, 24),
                consttable: CacheTableLayout::new(12, 6, 8, 28),
            },
            safe_frames: SafeFrameLayout::new(2, 5, 24, 8),
        }
    }
}
