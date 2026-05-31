//! Native Rust formatter for UCB BDD package statistics.
//!
//! The legacy C implementation receives a populated `bdd_stats` value and
//! prints the deterministic report. This module keeps that responsibility in
//! Rust-owned data structures and writer-based formatting.

use std::io;
use std::io::Write;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddCacheStats {
    pub hits: u32,
    pub misses: u32,
    pub collisions: u32,
    pub inserts: u32,
}

impl BddCacheStats {
    pub fn lookups(self) -> u32 {
        self.hits + self.misses
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddCacheSummary {
    pub hashtable: BddCacheStats,
    pub itetable: BddCacheStats,
    pub consttable: BddCacheStats,
    pub adhoc: BddCacheStats,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddReturnStats {
    pub trivial: u32,
    pub cached: u32,
    pub full: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddOperationStats {
    pub calls: u32,
    pub returns: BddReturnStats,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddBlockStats {
    pub total: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddNodeStats {
    pub used: u32,
    pub unused: u32,
    pub total: u32,
    pub peak: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddExternalPointerStats {
    pub used: u32,
    pub unused: u32,
    pub total: u32,
    pub blocks: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddGarbageCollectionStats {
    pub times: u32,
    pub nodes_collected: u32,
    pub runtime_millis: i64,
}

impl BddGarbageCollectionStats {
    pub fn runtime_seconds(self) -> f64 {
        self.runtime_millis as f64 / 1000.0
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddMemoryStats {
    pub first_sbrk: i32,
    pub last_sbrk: i32,
    pub manager: u32,
    pub nodes: u32,
    pub hashtable: u32,
    pub ext_ptrs: u32,
    pub ite_cache: u32,
    pub ite_const_cache: u32,
    pub adhoc_cache: u32,
    pub total: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddStats {
    pub cache: BddCacheSummary,
    pub ite_ops: BddOperationStats,
    pub ite_constant_ops: BddOperationStats,
    pub adhoc_ops: BddOperationStats,
    pub blocks: BddBlockStats,
    pub nodes: BddNodeStats,
    pub extptrs: BddExternalPointerStats,
    pub gc: BddGarbageCollectionStats,
    pub memory: BddMemoryStats,
}

pub fn bdd_percentage(numerator: u32, denominator: u32) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64 * 100.0
    }
}

pub fn format_bdd_stats(stats: &BddStats) -> String {
    let mut output = Vec::new();
    write_bdd_stats(stats, &mut output).expect("writing to String buffer cannot fail");
    String::from_utf8(output).expect("BDD stats report is ASCII")
}

pub fn write_bdd_stats<W>(stats: &BddStats, writer: &mut W) -> io::Result<()>
where
    W: Write,
{
    let total_hashtable_queries = stats.cache.hashtable.lookups();
    let total_itetable_lookups = stats.cache.itetable.lookups();
    let total_consttable_lookups = stats.cache.consttable.lookups();
    let total_adhoc_lookups = stats.cache.adhoc.lookups();

    write!(
        writer,
        "\
BDD Package Statistics

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
    total:  {:8} (find_or_add calls)

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
  manager:         {:9}
  bdd_nodes:       {:9}
  hashtable:       {:9}
  extptrs (bdd_t): {:9}
  ITE cache:       {:9}
  ITE_const cache: {:9}
  adhoc cache:     {:9}
  total:           {:9}
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
        bdd_percentage(stats.cache.hashtable.hits, total_hashtable_queries),
        stats.cache.hashtable.misses,
        bdd_percentage(stats.cache.hashtable.misses, total_hashtable_queries),
        total_hashtable_queries,
        stats.ite_ops.calls,
        stats.ite_constant_ops.calls,
        stats.adhoc_ops.calls,
        bdd_percentage(stats.ite_ops.returns.trivial, stats.ite_ops.calls),
        bdd_percentage(
            stats.ite_constant_ops.returns.trivial,
            stats.ite_constant_ops.calls
        ),
        bdd_percentage(stats.adhoc_ops.returns.trivial, stats.adhoc_ops.calls),
        bdd_percentage(stats.ite_ops.returns.cached, stats.ite_ops.calls),
        bdd_percentage(
            stats.ite_constant_ops.returns.cached,
            stats.ite_constant_ops.calls
        ),
        bdd_percentage(stats.adhoc_ops.returns.cached, stats.adhoc_ops.calls),
        bdd_percentage(stats.ite_ops.returns.full, stats.ite_ops.calls),
        bdd_percentage(
            stats.ite_constant_ops.returns.full,
            stats.ite_constant_ops.calls
        ),
        bdd_percentage(stats.adhoc_ops.returns.full, stats.adhoc_ops.calls),
        total_itetable_lookups,
        total_consttable_lookups,
        total_adhoc_lookups,
        bdd_percentage(stats.cache.itetable.misses, total_itetable_lookups),
        bdd_percentage(stats.cache.consttable.misses, total_consttable_lookups),
        bdd_percentage(stats.cache.adhoc.misses, total_adhoc_lookups),
        stats.cache.itetable.inserts,
        stats.cache.consttable.inserts,
        bdd_percentage(
            stats.cache.itetable.collisions,
            stats.cache.itetable.inserts
        ),
        bdd_percentage(
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
        stats.memory.ite_cache,
        stats.memory.ite_const_cache,
        stats.memory.adhoc_cache,
        stats.memory.total
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentage_returns_zero_for_empty_denominator() {
        assert_eq!(bdd_percentage(42, 0), 0.0);
        assert_eq!(bdd_percentage(1, 4), 25.0);
    }

    #[test]
    fn cache_lookup_totals_match_hits_plus_misses() {
        let cache = BddCacheStats {
            hits: 7,
            misses: 5,
            collisions: 3,
            inserts: 2,
        };

        assert_eq!(cache.lookups(), 12);
    }

    #[test]
    fn garbage_collection_runtime_is_reported_in_seconds() {
        let gc = BddGarbageCollectionStats {
            times: 2,
            nodes_collected: 11,
            runtime_millis: 1250,
        };

        assert_eq!(gc.runtime_seconds(), 1.25);
    }

    #[test]
    fn formatter_preserves_legacy_sections_and_percentages() {
        let stats = sample_stats();
        let report = format_bdd_stats(&stats);

        assert!(report.starts_with("BDD Package Statistics\n\n"));
        assert!(report.contains("Blocks (bdd_nodeBlock): 3\n"));
        assert!(report.contains("    hits:         30 (75.0%)\n"));
        assert!(report.contains("    misses:       10 (25.0%)\n"));
        assert!(report.contains(" Total calls:          10         20         25\n"));
        assert!(report.contains("    trivial:         10.0%      25.0%      20.0%\n"));
        assert!(report.contains("    cached:          30.0%      35.0%      20.0%\n"));
        assert!(report.contains("    full:            60.0%      40.0%      60.0%\n"));
        assert!(report.contains(" Total lookups:        10         20          4\n"));
        assert!(report.contains("    misses:          40.0%      40.0%      25.0%\n"));
        assert!(report.contains("    collisions:      40.0%      25.0%       --\n"));
        assert!(report.contains("    total time:  2.50 sec\n"));
        assert!(report.ends_with("  total:                 800\n"));
    }

    #[test]
    fn formatter_uses_zero_percentages_for_empty_counters() {
        let report = format_bdd_stats(&BddStats::default());

        assert!(report.contains("    hits:          0 ( 0.0%)\n"));
        assert!(report.contains("    trivial:          0.0%       0.0%       0.0%\n"));
        assert!(report.contains("    collisions:       0.0%       0.0%       --\n"));
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

        let error = write_bdd_stats(&BddStats::default(), &mut FailingWriter).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Other);
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_dependency_metadata_are_present() {
        let source = include_str!("print_stats.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }

    fn sample_stats() -> BddStats {
        BddStats {
            cache: BddCacheSummary {
                hashtable: BddCacheStats {
                    hits: 30,
                    misses: 10,
                    collisions: 0,
                    inserts: 0,
                },
                itetable: BddCacheStats {
                    hits: 6,
                    misses: 4,
                    collisions: 4,
                    inserts: 10,
                },
                consttable: BddCacheStats {
                    hits: 12,
                    misses: 8,
                    collisions: 2,
                    inserts: 8,
                },
                adhoc: BddCacheStats {
                    hits: 3,
                    misses: 1,
                    collisions: 0,
                    inserts: 0,
                },
            },
            ite_ops: BddOperationStats {
                calls: 10,
                returns: BddReturnStats {
                    trivial: 1,
                    cached: 3,
                    full: 6,
                },
            },
            ite_constant_ops: BddOperationStats {
                calls: 20,
                returns: BddReturnStats {
                    trivial: 5,
                    cached: 7,
                    full: 8,
                },
            },
            adhoc_ops: BddOperationStats {
                calls: 25,
                returns: BddReturnStats {
                    trivial: 5,
                    cached: 5,
                    full: 15,
                },
            },
            blocks: BddBlockStats { total: 3 },
            nodes: BddNodeStats {
                used: 4,
                unused: 5,
                total: 9,
                peak: 12,
            },
            extptrs: BddExternalPointerStats {
                used: 6,
                unused: 7,
                total: 13,
                blocks: 1,
            },
            gc: BddGarbageCollectionStats {
                times: 2,
                nodes_collected: 55,
                runtime_millis: 2500,
            },
            memory: BddMemoryStats {
                first_sbrk: 0,
                last_sbrk: 0,
                manager: 100,
                nodes: 200,
                hashtable: 300,
                ext_ptrs: 400,
                ite_cache: 500,
                ite_const_cache: 600,
                adhoc_cache: 700,
                total: 800,
            },
        }
    }
}
