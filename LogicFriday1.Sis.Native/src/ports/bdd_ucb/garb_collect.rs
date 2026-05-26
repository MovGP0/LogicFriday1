//! Native Rust model for UCB BDD stop-and-copy garbage collection.
//!
//! The legacy collector flips halfspaces, relocates root-reachable nodes, rebuilds
//! the unique table, and keeps cache entries only when every referenced node was
//! already forwarded by the root scan. This module models that behavior with
//! owned Rust handles and containers.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::time::Duration;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddNodeKey(usize);

impl BddNodeKey {
    pub fn value(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddPointer {
    node: BddNodeKey,
    complemented: bool,
}

impl BddPointer {
    pub fn regular(node: BddNodeKey) -> Self {
        Self {
            node,
            complemented: false,
        }
    }

    pub fn complemented(node: BddNodeKey) -> Self {
        Self {
            node,
            complemented: true,
        }
    }

    pub fn node(self) -> BddNodeKey {
        self.node
    }

    pub fn is_complemented(self) -> bool {
        self.complemented
    }

    pub fn with_node(self, node: BddNodeKey) -> Self {
        Self {
            node,
            complemented: self.complemented,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddNode {
    pub id: usize,
    pub then_child: Option<BddPointer>,
    pub else_child: Option<BddPointer>,
    pub unique_id: usize,
    pub age: usize,
}

impl BddNode {
    pub fn new(id: usize, then_child: Option<BddPointer>, else_child: Option<BddPointer>) -> Self {
        Self {
            id,
            then_child,
            else_child,
            unique_id: 0,
            age: 0,
        }
    }

    pub fn with_unique_id(mut self, unique_id: usize) -> Self {
        self.unique_id = unique_id;
        self
    }

    pub fn with_age(mut self, age: usize) -> Self {
        self.age = age;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
struct NodeSignature {
    id: usize,
    then_child: Option<BddPointer>,
    else_child: Option<BddPointer>,
}

impl From<&BddNode> for NodeSignature {
    fn from(node: &BddNode) -> Self {
        Self {
            id: node.id,
            then_child: node.then_child,
            else_child: node.else_child,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddHashCacheEntry {
    pub f: BddPointer,
    pub g: BddPointer,
    pub h: BddPointer,
    pub data: BddPointer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddConstValue {
    Unknown,
    Zero,
    One,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddConstCacheEntry {
    pub f: BddPointer,
    pub g: BddPointer,
    pub h: BddPointer,
    pub data: BddConstValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddAdhocCacheEntry {
    pub f: BddPointer,
    pub g: BddPointer,
    pub data: Option<BddPointer>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddHashCache {
    enabled: bool,
    invalidate_on_gc: bool,
    buckets: Vec<Option<BddHashCacheEntry>>,
}

impl BddHashCache {
    pub fn new(bucket_count: usize) -> Self {
        Self {
            enabled: true,
            invalidate_on_gc: false,
            buckets: vec![None; bucket_count.max(1)],
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_invalidate_on_gc(&mut self, invalidate_on_gc: bool) {
        self.invalidate_on_gc = invalidate_on_gc;
    }

    pub fn insert_at(&mut self, bucket: usize, entry: BddHashCacheEntry) {
        let bucket = bucket % self.buckets.len();
        self.buckets[bucket] = Some(entry);
    }

    pub fn entries(&self) -> impl Iterator<Item = &BddHashCacheEntry> {
        self.buckets.iter().filter_map(Option::as_ref)
    }

    pub fn entry_count(&self) -> usize {
        self.entries().count()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddConstCache {
    enabled: bool,
    invalidate_on_gc: bool,
    buckets: Vec<Option<BddConstCacheEntry>>,
}

impl BddConstCache {
    pub fn new(bucket_count: usize) -> Self {
        Self {
            enabled: true,
            invalidate_on_gc: false,
            buckets: vec![None; bucket_count.max(1)],
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_invalidate_on_gc(&mut self, invalidate_on_gc: bool) {
        self.invalidate_on_gc = invalidate_on_gc;
    }

    pub fn insert_at(&mut self, bucket: usize, entry: BddConstCacheEntry) {
        let bucket = bucket % self.buckets.len();
        self.buckets[bucket] = Some(entry);
    }

    pub fn entries(&self) -> impl Iterator<Item = &BddConstCacheEntry> {
        self.buckets.iter().filter_map(Option::as_ref)
    }

    pub fn entry_count(&self) -> usize {
        self.entries().count()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddAdhocCache {
    enabled: bool,
    entries: Vec<BddAdhocCacheEntry>,
}

impl BddAdhocCache {
    pub fn new() -> Self {
        Self {
            enabled: true,
            entries: Vec::new(),
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn insert(&mut self, entry: BddAdhocCacheEntry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[BddAdhocCacheEntry] {
        &self.entries
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for BddAdhocCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddCaches {
    pub ite: BddHashCache,
    pub constant: BddConstCache,
    pub adhoc: BddAdhocCache,
}

impl BddCaches {
    pub fn new(bucket_count: usize) -> Self {
        Self {
            ite: BddHashCache::new(bucket_count),
            constant: BddConstCache::new(bucket_count),
            adhoc: BddAdhocCache::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddGcStats {
    pub collections: usize,
    pub nodes_collected: usize,
    pub runtime: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddGcReport {
    pub skipped: bool,
    pub previous_nodes_used: usize,
    pub nodes_used: usize,
    pub nodes_collected: usize,
    pub hash_cache_entries: usize,
    pub const_cache_entries: usize,
    pub adhoc_cache_entries: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddGcError {
    AlreadyRunning,
    OpenGenerators { count: usize },
    MissingNode(BddNodeKey),
}

impl fmt::Display for BddGcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyRunning => {
                write!(formatter, "BDD garbage collection is already running")
            }
            Self::OpenGenerators { count } => {
                write!(
                    formatter,
                    "BDD garbage collection cannot run with {count} open generators"
                )
            }
            Self::MissingNode(node) => {
                write!(formatter, "BDD node {} is missing", node.value())
            }
        }
    }
}

impl Error for BddGcError {}

#[derive(Clone, Debug)]
pub struct BddGcManager {
    gc_enabled: bool,
    busy: bool,
    open_generators: usize,
    one: Option<BddPointer>,
    external_refs: Vec<Option<BddPointer>>,
    internal_refs: Vec<Option<BddPointer>>,
    nodes: Vec<BddNode>,
    unique_table: HashMap<NodeSignature, BddNodeKey>,
    pub caches: BddCaches,
    pub stats: BddGcStats,
}

impl BddGcManager {
    pub fn new(cache_bucket_count: usize) -> Self {
        Self {
            gc_enabled: true,
            busy: false,
            open_generators: 0,
            one: None,
            external_refs: Vec::new(),
            internal_refs: Vec::new(),
            nodes: Vec::new(),
            unique_table: HashMap::new(),
            caches: BddCaches::new(cache_bucket_count),
            stats: BddGcStats::default(),
        }
    }

    pub fn set_gc_mode(&mut self, no_gc: bool) {
        self.gc_enabled = !no_gc;
    }

    pub fn gc_enabled(&self) -> bool {
        self.gc_enabled
    }

    pub fn set_open_generators(&mut self, open_generators: usize) {
        self.open_generators = open_generators;
    }

    pub fn set_busy_for_test(&mut self, busy: bool) {
        self.busy = busy;
    }

    pub fn add_node(&mut self, node: BddNode) -> BddNodeKey {
        let key = BddNodeKey(self.nodes.len());
        self.unique_table.insert(NodeSignature::from(&node), key);
        self.nodes.push(node);
        key
    }

    pub fn node(&self, key: BddNodeKey) -> Option<&BddNode> {
        self.nodes.get(key.0)
    }

    pub fn nodes_used(&self) -> usize {
        self.nodes.len()
    }

    pub fn unique_table_len(&self) -> usize {
        self.unique_table.len()
    }

    pub fn set_one(&mut self, pointer: Option<BddPointer>) {
        self.one = pointer;
    }

    pub fn one(&self) -> Option<BddPointer> {
        self.one
    }

    pub fn add_external_ref(&mut self, pointer: Option<BddPointer>) {
        self.external_refs.push(pointer);
    }

    pub fn external_ref(&self, index: usize) -> Option<BddPointer> {
        self.external_refs.get(index).copied().flatten()
    }

    pub fn add_internal_ref(&mut self, pointer: Option<BddPointer>) {
        self.internal_refs.push(pointer);
    }

    pub fn internal_ref(&self, index: usize) -> Option<BddPointer> {
        self.internal_refs.get(index).copied().flatten()
    }

    pub fn garbage_collect(&mut self) -> Result<BddGcReport, BddGcError> {
        if !self.gc_enabled {
            let used = self.nodes_used();
            return Ok(BddGcReport {
                skipped: true,
                previous_nodes_used: used,
                nodes_used: used,
                nodes_collected: 0,
                hash_cache_entries: self.caches.ite.entry_count(),
                const_cache_entries: self.caches.constant.entry_count(),
                adhoc_cache_entries: self.caches.adhoc.entry_count(),
            });
        }

        if self.busy {
            return Err(BddGcError::AlreadyRunning);
        }

        if self.open_generators > 0 {
            return Err(BddGcError::OpenGenerators {
                count: self.open_generators,
            });
        }

        self.busy = true;
        let result = self.collect_enabled();
        self.busy = false;
        result
    }

    fn collect_enabled(&mut self) -> Result<BddGcReport, BddGcError> {
        let start = Instant::now();
        let previous_nodes_used = self.nodes_used();
        let cache_modes = CacheModes::capture(&self.caches);
        self.caches.ite.enabled = false;
        self.caches.constant.enabled = false;
        self.caches.adhoc.enabled = false;

        let old_nodes = std::mem::take(&mut self.nodes);
        self.unique_table.clear();
        let mut relocator = Relocator::new(old_nodes);

        self.one = relocator.relocate_pointer(self.one)?;

        for pointer in &mut self.external_refs {
            *pointer = relocator.relocate_pointer(*pointer)?;
        }

        for pointer in &mut self.internal_refs {
            *pointer = relocator.relocate_pointer(*pointer)?;
        }

        let mut scan_index = 0;
        while scan_index < relocator.new_nodes.len() {
            let then_child = relocator.new_nodes[scan_index].then_child;
            let else_child = relocator.new_nodes[scan_index].else_child;
            relocator.new_nodes[scan_index].then_child = relocator.relocate_pointer(then_child)?;
            relocator.new_nodes[scan_index].else_child = relocator.relocate_pointer(else_child)?;
            scan_index += 1;
        }

        self.nodes = relocator.new_nodes;
        self.rebuild_unique_table();
        self.scan_caches(&relocator.forwarded)?;
        cache_modes.restore(&mut self.caches);

        let nodes_used = self.nodes_used();
        let nodes_collected = previous_nodes_used.saturating_sub(nodes_used);
        self.stats.collections += 1;
        self.stats.nodes_collected += nodes_collected;
        self.stats.runtime += start.elapsed();

        Ok(BddGcReport {
            skipped: false,
            previous_nodes_used,
            nodes_used,
            nodes_collected,
            hash_cache_entries: self.caches.ite.entry_count(),
            const_cache_entries: self.caches.constant.entry_count(),
            adhoc_cache_entries: self.caches.adhoc.entry_count(),
        })
    }

    fn rebuild_unique_table(&mut self) {
        self.unique_table.clear();
        for (index, node) in self.nodes.iter().enumerate() {
            self.unique_table
                .insert(NodeSignature::from(node), BddNodeKey(index));
        }
    }

    fn scan_caches(&mut self, forwarded: &[Option<BddNodeKey>]) -> Result<(), BddGcError> {
        self.scan_hash_cache(forwarded)?;
        self.scan_const_cache(forwarded)?;
        self.scan_adhoc_cache(forwarded)?;
        Ok(())
    }

    fn scan_hash_cache(&mut self, forwarded: &[Option<BddNodeKey>]) -> Result<(), BddGcError> {
        if self.caches.ite.invalidate_on_gc {
            self.caches.ite.buckets.fill(None);
            return Ok(());
        }

        let bucket_count = self.caches.ite.buckets.len();
        let old_buckets = std::mem::replace(&mut self.caches.ite.buckets, vec![None; bucket_count]);

        for entry in old_buckets.into_iter().flatten() {
            if forwarded_pointer(entry.f, forwarded).is_some()
                && forwarded_pointer(entry.g, forwarded).is_some()
                && forwarded_pointer(entry.h, forwarded).is_some()
                && forwarded_pointer(entry.data, forwarded).is_some()
            {
                let relocated = BddHashCacheEntry {
                    f: relocate_cache_pointer(entry.f, forwarded)?,
                    g: relocate_cache_pointer(entry.g, forwarded)?,
                    h: relocate_cache_pointer(entry.h, forwarded)?,
                    data: relocate_cache_pointer(entry.data, forwarded)?,
                };
                let bucket = hash_ite(
                    relocated.f,
                    relocated.g,
                    relocated.h,
                    self.caches.ite.buckets.len(),
                );
                if self.caches.ite.buckets[bucket].is_none() {
                    self.caches.ite.buckets[bucket] = Some(relocated);
                }
            }
        }

        Ok(())
    }

    fn scan_const_cache(&mut self, forwarded: &[Option<BddNodeKey>]) -> Result<(), BddGcError> {
        if self.caches.constant.invalidate_on_gc {
            self.caches.constant.buckets.fill(None);
            return Ok(());
        }

        let bucket_count = self.caches.constant.buckets.len();
        let old_buckets =
            std::mem::replace(&mut self.caches.constant.buckets, vec![None; bucket_count]);

        for entry in old_buckets.into_iter().flatten() {
            if entry.data != BddConstValue::Unknown
                && forwarded_pointer(entry.f, forwarded).is_some()
                && forwarded_pointer(entry.g, forwarded).is_some()
                && forwarded_pointer(entry.h, forwarded).is_some()
            {
                let relocated = BddConstCacheEntry {
                    f: relocate_cache_pointer(entry.f, forwarded)?,
                    g: relocate_cache_pointer(entry.g, forwarded)?,
                    h: relocate_cache_pointer(entry.h, forwarded)?,
                    data: entry.data,
                };
                let bucket = hash_ite(
                    relocated.f,
                    relocated.g,
                    relocated.h,
                    self.caches.constant.buckets.len(),
                );
                if self.caches.constant.buckets[bucket].is_none() {
                    self.caches.constant.buckets[bucket] = Some(relocated);
                }
            }
        }

        Ok(())
    }

    fn scan_adhoc_cache(&mut self, forwarded: &[Option<BddNodeKey>]) -> Result<(), BddGcError> {
        let old_entries = std::mem::take(&mut self.caches.adhoc.entries);

        for entry in old_entries {
            if forwarded_pointer(entry.f, forwarded).is_some()
                && forwarded_pointer(entry.g, forwarded).is_some()
                && entry
                    .data
                    .map(|pointer| forwarded_pointer(pointer, forwarded).is_some())
                    .unwrap_or(true)
            {
                self.caches.adhoc.entries.push(BddAdhocCacheEntry {
                    f: relocate_cache_pointer(entry.f, forwarded)?,
                    g: relocate_cache_pointer(entry.g, forwarded)?,
                    data: entry
                        .data
                        .map(|pointer| relocate_cache_pointer(pointer, forwarded))
                        .transpose()?,
                });
            }
        }

        Ok(())
    }
}

pub fn bdd_set_gc_mode(manager: &mut BddGcManager, no_gc: bool) {
    manager.set_gc_mode(no_gc);
}

pub fn bdd_garbage_collect(manager: &mut BddGcManager) -> Result<BddGcReport, BddGcError> {
    manager.garbage_collect()
}

#[derive(Clone, Copy, Debug)]
struct CacheModes {
    ite: bool,
    constant: bool,
    adhoc: bool,
}

impl CacheModes {
    fn capture(caches: &BddCaches) -> Self {
        Self {
            ite: caches.ite.enabled,
            constant: caches.constant.enabled,
            adhoc: caches.adhoc.enabled,
        }
    }

    fn restore(self, caches: &mut BddCaches) {
        caches.ite.enabled = self.ite;
        caches.constant.enabled = self.constant;
        caches.adhoc.enabled = self.adhoc;
    }
}

struct Relocator {
    old_nodes: Vec<BddNode>,
    new_nodes: Vec<BddNode>,
    forwarded: Vec<Option<BddNodeKey>>,
}

impl Relocator {
    fn new(old_nodes: Vec<BddNode>) -> Self {
        let forwarded = vec![None; old_nodes.len()];
        Self {
            old_nodes,
            new_nodes: Vec::new(),
            forwarded,
        }
    }

    fn relocate_pointer(
        &mut self,
        pointer: Option<BddPointer>,
    ) -> Result<Option<BddPointer>, BddGcError> {
        pointer.map(|pointer| self.relocate(pointer)).transpose()
    }

    fn relocate(&mut self, pointer: BddPointer) -> Result<BddPointer, BddGcError> {
        let old_index = pointer.node.0;
        if old_index >= self.old_nodes.len() {
            return Err(BddGcError::MissingNode(pointer.node));
        }

        let new_key = match self.forwarded[old_index] {
            Some(new_key) => new_key,
            None => {
                let new_key = BddNodeKey(self.new_nodes.len());
                self.new_nodes.push(self.old_nodes[old_index].clone());
                self.forwarded[old_index] = Some(new_key);
                new_key
            }
        };

        Ok(pointer.with_node(new_key))
    }
}

fn forwarded_pointer(pointer: BddPointer, forwarded: &[Option<BddNodeKey>]) -> Option<BddNodeKey> {
    forwarded.get(pointer.node.0).copied().flatten()
}

fn relocate_cache_pointer(
    pointer: BddPointer,
    forwarded: &[Option<BddNodeKey>],
) -> Result<BddPointer, BddGcError> {
    forwarded_pointer(pointer, forwarded)
        .map(|node| pointer.with_node(node))
        .ok_or(BddGcError::MissingNode(pointer.node))
}

fn hash_ite(f: BddPointer, g: BddPointer, h: BddPointer, bucket_count: usize) -> usize {
    let f_phase = usize::from(f.complemented);
    let g_phase = usize::from(g.complemented);
    let h_phase = usize::from(h.complemented);
    (f.node.0 * 17 + f_phase * 3 + g.node.0 * 31 + g_phase * 5 + h.node.0 * 43 + h_phase * 7)
        % bucket_count.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_gc_mode_uses_legacy_inverted_flag() {
        let mut manager = BddGcManager::new(4);

        bdd_set_gc_mode(&mut manager, true);
        assert!(!manager.gc_enabled());

        bdd_set_gc_mode(&mut manager, false);
        assert!(manager.gc_enabled());
    }

    #[test]
    fn disabled_collector_skips_without_mutating_roots_or_stats() {
        let mut manager = BddGcManager::new(4);
        let node = manager.add_node(BddNode::new(1, None, None));
        manager.set_one(Some(BddPointer::regular(node)));
        manager.set_gc_mode(true);

        let report = bdd_garbage_collect(&mut manager).unwrap();

        assert!(report.skipped);
        assert_eq!(report.nodes_used, 1);
        assert_eq!(manager.one(), Some(BddPointer::regular(node)));
        assert_eq!(manager.stats.collections, 0);
    }

    #[test]
    fn rejects_reentrant_collection_and_open_generators() {
        let mut manager = BddGcManager::new(4);
        manager.set_busy_for_test(true);
        assert_eq!(
            bdd_garbage_collect(&mut manager),
            Err(BddGcError::AlreadyRunning)
        );

        manager.set_busy_for_test(false);
        manager.set_open_generators(2);
        assert_eq!(
            bdd_garbage_collect(&mut manager),
            Err(BddGcError::OpenGenerators { count: 2 })
        );
    }

    #[test]
    fn collection_relocates_transitive_roots_and_collects_unreachable_nodes() {
        let mut manager = BddGcManager::new(8);
        let leaf = manager.add_node(BddNode::new(3, None, None).with_unique_id(30));
        let dead = manager.add_node(BddNode::new(9, None, None).with_unique_id(90));
        let root = manager.add_node(
            BddNode::new(
                1,
                Some(BddPointer::regular(leaf)),
                Some(BddPointer::complemented(leaf)),
            )
            .with_unique_id(10)
            .with_age(4),
        );
        manager.set_one(Some(BddPointer::complemented(root)));
        manager.add_external_ref(Some(BddPointer::regular(dead)));
        manager.add_internal_ref(Some(BddPointer::regular(root)));

        let report = bdd_garbage_collect(&mut manager).unwrap();

        assert!(!report.skipped);
        assert_eq!(report.previous_nodes_used, 3);
        assert_eq!(report.nodes_used, 3);
        assert_eq!(report.nodes_collected, 0);
        assert_eq!(manager.one().unwrap().is_complemented(), true);
        assert_eq!(manager.one().unwrap().node(), BddNodeKey(0));
        assert_eq!(manager.internal_ref(0).unwrap().node(), BddNodeKey(0));
        assert_eq!(manager.external_ref(0).unwrap().node(), BddNodeKey(1));
        assert_eq!(manager.node(BddNodeKey(0)).unwrap().unique_id, 10);
        assert_eq!(manager.node(BddNodeKey(0)).unwrap().age, 4);
        assert_eq!(
            manager.node(BddNodeKey(0)).unwrap().then_child,
            Some(BddPointer::regular(BddNodeKey(2)))
        );
        assert_eq!(manager.unique_table_len(), 3);
        assert_eq!(manager.stats.collections, 1);
    }

    #[test]
    fn unreachable_nodes_are_collected_when_not_in_roots_or_children() {
        let mut manager = BddGcManager::new(8);
        let live = manager.add_node(BddNode::new(1, None, None));
        manager.add_node(BddNode::new(2, None, None));
        manager.set_one(Some(BddPointer::regular(live)));

        let report = bdd_garbage_collect(&mut manager).unwrap();

        assert_eq!(report.previous_nodes_used, 2);
        assert_eq!(report.nodes_used, 1);
        assert_eq!(report.nodes_collected, 1);
        assert_eq!(manager.stats.nodes_collected, 1);
        assert_eq!(manager.one(), Some(BddPointer::regular(BddNodeKey(0))));
    }

    #[test]
    fn hash_cache_keeps_only_fully_forwarded_entries_and_restores_modes() {
        let mut manager = BddGcManager::new(8);
        let f = manager.add_node(BddNode::new(1, None, None));
        let g = manager.add_node(BddNode::new(2, None, None));
        let h = manager.add_node(BddNode::new(3, None, None));
        let data = manager.add_node(BddNode::new(4, None, None));
        let stale = manager.add_node(BddNode::new(5, None, None));
        manager.set_one(Some(BddPointer::regular(f)));
        manager.add_internal_ref(Some(BddPointer::regular(g)));
        manager.add_internal_ref(Some(BddPointer::regular(h)));
        manager.add_internal_ref(Some(BddPointer::regular(data)));
        manager.caches.ite.set_enabled(true);
        manager.caches.ite.insert_at(
            0,
            BddHashCacheEntry {
                f: BddPointer::regular(f),
                g: BddPointer::complemented(g),
                h: BddPointer::regular(h),
                data: BddPointer::regular(data),
            },
        );
        manager.caches.ite.insert_at(
            1,
            BddHashCacheEntry {
                f: BddPointer::regular(f),
                g: BddPointer::regular(g),
                h: BddPointer::regular(h),
                data: BddPointer::regular(stale),
            },
        );

        let report = bdd_garbage_collect(&mut manager).unwrap();

        assert_eq!(report.hash_cache_entries, 1);
        assert!(manager.caches.ite.is_enabled());
        let kept = manager.caches.ite.entries().next().unwrap();
        assert_eq!(kept.f.node(), BddNodeKey(0));
        assert_eq!(kept.g, BddPointer::complemented(BddNodeKey(1)));
        assert_eq!(kept.h.node(), BddNodeKey(2));
        assert_eq!(kept.data.node(), BddNodeKey(3));
    }

    #[test]
    fn const_cache_drops_unknown_values_and_can_invalidate() {
        let mut manager = BddGcManager::new(8);
        let f = manager.add_node(BddNode::new(1, None, None));
        let g = manager.add_node(BddNode::new(2, None, None));
        let h = manager.add_node(BddNode::new(3, None, None));
        manager.set_one(Some(BddPointer::regular(f)));
        manager.add_internal_ref(Some(BddPointer::regular(g)));
        manager.add_internal_ref(Some(BddPointer::regular(h)));
        manager.caches.constant.insert_at(
            0,
            BddConstCacheEntry {
                f: BddPointer::regular(f),
                g: BddPointer::regular(g),
                h: BddPointer::regular(h),
                data: BddConstValue::One,
            },
        );
        manager.caches.constant.insert_at(
            1,
            BddConstCacheEntry {
                f: BddPointer::regular(f),
                g: BddPointer::regular(g),
                h: BddPointer::regular(h),
                data: BddConstValue::Unknown,
            },
        );

        let report = bdd_garbage_collect(&mut manager).unwrap();
        assert_eq!(report.const_cache_entries, 1);

        manager.caches.constant.set_invalidate_on_gc(true);
        let report = bdd_garbage_collect(&mut manager).unwrap();
        assert_eq!(report.const_cache_entries, 0);
    }

    #[test]
    fn adhoc_cache_allows_empty_data_but_drops_stale_data() {
        let mut manager = BddGcManager::new(8);
        let f = manager.add_node(BddNode::new(1, None, None));
        let g = manager.add_node(BddNode::new(2, None, None));
        let data = manager.add_node(BddNode::new(3, None, None));
        let stale = manager.add_node(BddNode::new(4, None, None));
        manager.set_one(Some(BddPointer::regular(f)));
        manager.add_internal_ref(Some(BddPointer::regular(g)));
        manager.add_internal_ref(Some(BddPointer::regular(data)));
        manager.caches.adhoc.insert(BddAdhocCacheEntry {
            f: BddPointer::regular(f),
            g: BddPointer::regular(g),
            data: None,
        });
        manager.caches.adhoc.insert(BddAdhocCacheEntry {
            f: BddPointer::regular(f),
            g: BddPointer::regular(g),
            data: Some(BddPointer::complemented(data)),
        });
        manager.caches.adhoc.insert(BddAdhocCacheEntry {
            f: BddPointer::regular(f),
            g: BddPointer::regular(g),
            data: Some(BddPointer::regular(stale)),
        });

        let report = bdd_garbage_collect(&mut manager).unwrap();

        assert_eq!(report.adhoc_cache_entries, 2);
        assert_eq!(manager.caches.adhoc.entries()[0].data, None);
        assert_eq!(
            manager.caches.adhoc.entries()[1].data,
            Some(BddPointer::complemented(BddNodeKey(2)))
        );
    }

    #[test]
    fn invalid_root_reports_missing_node_and_clears_busy_flag() {
        let mut manager = BddGcManager::new(4);
        manager.set_one(Some(BddPointer::regular(BddNodeKey(4))));

        assert_eq!(
            bdd_garbage_collect(&mut manager),
            Err(BddGcError::MissingNode(BddNodeKey(4)))
        );

        assert_eq!(
            bdd_garbage_collect(&mut manager),
            Err(BddGcError::MissingNode(BddNodeKey(4)))
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_dependency_metadata_are_present() {
        let text = include_str!("garb_collect.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
        assert!(!text.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }
}
