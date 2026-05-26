//! Native Rust heap allocation model for the SIS UCB BDD `new_node.c` unit.
//!
//! The original C code allocates BDD nodes from halfspace block lists, reuses
//! free blocks before growing the heap, and preserves debug identity while the
//! garbage collector relocates an existing node. This port keeps those runtime
//! rules in safe owned Rust data rather than exposing per-file C ABI entry
//! points.

use std::error::Error;
use std::fmt;

pub const DEFAULT_NODE_BLOCK_CAPACITY: usize = 1024;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddNodeHandle {
    block: usize,
    index: usize,
}

impl BddNodeHandle {
    pub fn block(self) -> usize {
        self.block
    }

    pub fn index(self) -> usize {
        self.index
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddNodeLinks {
    pub then_child: Option<BddNodeHandle>,
    pub else_child: Option<BddNodeHandle>,
}

impl BddNodeLinks {
    pub fn new(then_child: Option<BddNodeHandle>, else_child: Option<BddNodeHandle>) -> Self {
        Self {
            then_child,
            else_child,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddNode {
    pub variable_id: usize,
    pub links: BddNodeLinks,
    pub next: Option<BddNodeHandle>,
    pub age: usize,
    pub unique_id: usize,
    pub halfspace: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OldNodeDebug {
    pub age: usize,
    pub unique_id: usize,
}

impl From<&BddNode> for OldNodeDebug {
    fn from(node: &BddNode) -> Self {
        Self {
            age: node.age,
            unique_id: node.unique_id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeBlock {
    used: usize,
    nodes: Vec<Option<BddNode>>,
}

impl NodeBlock {
    fn new(capacity: usize) -> Self {
        Self {
            used: 0,
            nodes: vec![None; capacity],
        }
    }

    pub fn used(&self) -> usize {
        self.used
    }

    pub fn capacity(&self) -> usize {
        self.nodes.len()
    }

    pub fn node(&self, index: usize) -> Option<&BddNode> {
        self.nodes.get(index).and_then(Option::as_ref)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Halfspace {
    free: Vec<usize>,
    inuse: Vec<usize>,
}

impl Halfspace {
    pub fn free_blocks(&self) -> &[usize] {
        &self.free
    }

    pub fn inuse_blocks(&self) -> &[usize] {
        &self.inuse
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AllocationCursor {
    block: Option<usize>,
    index: usize,
}

impl AllocationCursor {
    pub fn block(self) -> Option<usize> {
        self.block
    }

    pub fn index(self) -> usize {
        self.index
    }
}

impl Default for AllocationCursor {
    fn default() -> Self {
        Self {
            block: None,
            index: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NodeStats {
    pub used: usize,
    pub peak: usize,
    pub unused: usize,
    pub total: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BlockStats {
    pub total: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HeapStats {
    pub nodes: NodeStats,
    pub blocks: BlockStats,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GarbageCollectionConfig {
    pub halfspace: usize,
    pub node_ratio: usize,
    pub age: usize,
    pub next_unique_id: usize,
}

impl Default for GarbageCollectionConfig {
    fn default() -> Self {
        Self {
            halfspace: 0,
            node_ratio: 2,
            age: 0,
            next_unique_id: 1,
        }
    }
}

pub trait GarbageCollector {
    fn collect_garbage(&mut self, heap: &mut BddNodeHeap) -> Result<(), BddNewNodeError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddNodeHeap {
    block_capacity: usize,
    init_node_blocks: usize,
    allocation_limit_blocks: Option<usize>,
    half: [Halfspace; 2],
    blocks: Vec<NodeBlock>,
    pointer: AllocationCursor,
    stats: HeapStats,
    gc: GarbageCollectionConfig,
    lifespan_trace: Vec<usize>,
}

impl BddNodeHeap {
    pub fn new(block_capacity: usize, init_node_blocks: usize) -> Result<Self, BddNewNodeError> {
        if block_capacity == 0 {
            return Err(BddNewNodeError::ZeroBlockCapacity);
        }

        if init_node_blocks == 0 {
            return Err(BddNewNodeError::ZeroInitialBlocks);
        }

        Ok(Self {
            block_capacity,
            init_node_blocks,
            allocation_limit_blocks: None,
            half: [Halfspace::default(), Halfspace::default()],
            blocks: Vec::new(),
            pointer: AllocationCursor::default(),
            stats: HeapStats::default(),
            gc: GarbageCollectionConfig::default(),
            lifespan_trace: Vec::new(),
        })
    }

    pub fn with_allocation_limit_blocks(mut self, limit: usize) -> Self {
        self.allocation_limit_blocks = Some(limit);
        self
    }

    pub fn cursor(&self) -> AllocationCursor {
        self.pointer
    }

    pub fn stats(&self) -> HeapStats {
        self.stats
    }

    pub fn halfspace(&self, halfspace: usize) -> Result<&Halfspace, BddNewNodeError> {
        self.half
            .get(halfspace)
            .ok_or(BddNewNodeError::InvalidHalfspace(halfspace))
    }

    pub fn active_halfspace(&self) -> usize {
        self.gc.halfspace
    }

    pub fn set_active_halfspace(&mut self, halfspace: usize) -> Result<(), BddNewNodeError> {
        if halfspace >= self.half.len() {
            return Err(BddNewNodeError::InvalidHalfspace(halfspace));
        }

        self.gc.halfspace = halfspace;
        self.pointer = AllocationCursor::default();
        Ok(())
    }

    pub fn set_node_ratio(&mut self, node_ratio: usize) {
        self.gc.node_ratio = node_ratio.max(1);
    }

    pub fn set_age(&mut self, age: usize) {
        self.gc.age = age;
    }

    pub fn lifespan_trace(&self) -> &[usize] {
        &self.lifespan_trace
    }

    pub fn node(&self, handle: BddNodeHandle) -> Option<&BddNode> {
        self.blocks
            .get(handle.block)
            .and_then(|block| block.node(handle.index))
    }

    pub fn new_node(
        &mut self,
        variable_id: usize,
        links: BddNodeLinks,
        old: Option<OldNodeDebug>,
        collector: &mut impl GarbageCollector,
    ) -> Result<BddNodeHandle, BddNewNodeError> {
        if self.cursor_is_full() {
            if old.is_some() && self.half[self.gc.halfspace].free.is_empty() {
                return Err(BddNewNodeError::RelocationWithoutFreeBlock);
            }

            self.get_node_block(collector)?;
        }

        let block = self
            .pointer
            .block
            .ok_or(BddNewNodeError::MissingAllocationBlock)?;
        let index = self.pointer.index;
        let debug = old.unwrap_or_else(|| {
            let unique_id = self.gc.next_unique_id;
            self.gc.next_unique_id += 1;
            self.lifespan_trace.push(unique_id);
            OldNodeDebug {
                age: self.gc.age,
                unique_id,
            }
        });
        let node = BddNode {
            variable_id,
            links,
            next: None,
            age: debug.age,
            unique_id: debug.unique_id,
            halfspace: self.gc.halfspace,
        };

        self.blocks[block].nodes[index] = Some(node);
        self.blocks[block].used += 1;
        self.pointer.index += 1;
        self.stats.nodes.used += 1;
        self.stats.nodes.peak = self.stats.nodes.peak.max(self.stats.nodes.used);
        self.stats.nodes.unused = self.stats.nodes.unused.saturating_sub(1);

        Ok(BddNodeHandle { block, index })
    }

    fn cursor_is_full(&self) -> bool {
        match self.pointer.block {
            None => true,
            Some(block) => self.pointer.index >= self.blocks[block].capacity(),
        }
    }

    fn get_node_block(
        &mut self,
        collector: &mut impl GarbageCollector,
    ) -> Result<(), BddNewNodeError> {
        if !self.half[self.gc.halfspace].free.is_empty() {
            return self.get_block_from_free_list();
        }

        if self.half[self.gc.halfspace].inuse.is_empty() {
            let mut request_optional = false;
            for _ in 0..self.init_node_blocks {
                if !self.add_block_to_free_list(request_optional)? {
                    break;
                }

                request_optional = true;
            }

            return self.get_block_from_free_list();
        }

        collector.collect_garbage(self)?;

        let mut request_optional =
            !self.cursor_is_full() || !self.half[self.gc.halfspace].free.is_empty();
        while self.stats.nodes.used > self.stats.nodes.unused.saturating_mul(self.gc.node_ratio) {
            if !self.add_block_to_free_list(request_optional)? {
                break;
            }

            request_optional = true;
        }

        if self.cursor_is_full() {
            self.get_block_from_free_list()?;
        }

        Ok(())
    }

    fn get_block_from_free_list(&mut self) -> Result<(), BddNewNodeError> {
        let block = self.half[self.gc.halfspace]
            .free
            .pop()
            .ok_or(BddNewNodeError::MissingFreeBlock)?;
        self.blocks[block].used = 0;
        for slot in &mut self.blocks[block].nodes {
            *slot = None;
        }

        self.half[self.gc.halfspace].inuse.push(block);
        self.pointer = AllocationCursor {
            block: Some(block),
            index: 0,
        };

        Ok(())
    }

    fn add_block_to_free_list(&mut self, optional: bool) -> Result<bool, BddNewNodeError> {
        if self.will_exceed_block_limit() {
            return if optional {
                Ok(false)
            } else {
                Err(BddNewNodeError::MemoryLimitExceeded)
            };
        }

        for halfspace in 0..self.half.len() {
            let block = self.blocks.len();
            self.blocks.push(NodeBlock::new(self.block_capacity));
            self.half[halfspace].free.push(block);
        }

        self.stats.nodes.unused += self.block_capacity;
        self.stats.nodes.total += self.block_capacity;
        self.stats.blocks.total += 1;
        Ok(true)
    }

    fn will_exceed_block_limit(&self) -> bool {
        self.allocation_limit_blocks
            .is_some_and(|limit| self.stats.blocks.total + 1 > limit)
    }
}

impl Default for BddNodeHeap {
    fn default() -> Self {
        Self::new(DEFAULT_NODE_BLOCK_CAPACITY, 1).expect("default BDD node heap is valid")
    }
}

#[derive(Default)]
pub struct NoopGarbageCollector;

impl GarbageCollector for NoopGarbageCollector {
    fn collect_garbage(&mut self, _heap: &mut BddNodeHeap) -> Result<(), BddNewNodeError> {
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddNewNodeError {
    ZeroBlockCapacity,
    ZeroInitialBlocks,
    InvalidHalfspace(usize),
    MemoryLimitExceeded,
    MissingAllocationBlock,
    MissingFreeBlock,
    RelocationWithoutFreeBlock,
}

impl fmt::Display for BddNewNodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroBlockCapacity => {
                write!(f, "BDD node block capacity must be greater than zero")
            }
            Self::ZeroInitialBlocks => {
                write!(f, "initial BDD node block count must be greater than zero")
            }
            Self::InvalidHalfspace(halfspace) => write!(f, "invalid BDD halfspace {halfspace}"),
            Self::MemoryLimitExceeded => write!(f, "BDD node heap memory limit exceeded"),
            Self::MissingAllocationBlock => {
                write!(f, "BDD node heap has no active allocation block")
            }
            Self::MissingFreeBlock => write!(f, "BDD node heap free list is empty"),
            Self::RelocationWithoutFreeBlock => {
                write!(f, "BDD GC relocation requires a free node block")
            }
        }
    }
}

impl Error for BddNewNodeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingCollector {
        calls: usize,
    }

    impl GarbageCollector for RecordingCollector {
        fn collect_garbage(&mut self, _heap: &mut BddNodeHeap) -> Result<(), BddNewNodeError> {
            self.calls += 1;
            Ok(())
        }
    }

    fn links() -> BddNodeLinks {
        BddNodeLinks::new(None, None)
    }

    #[test]
    fn first_allocation_seeds_free_lists_and_uses_active_halfspace_block() {
        let mut heap = BddNodeHeap::new(2, 2).unwrap();
        let mut collector = NoopGarbageCollector;

        let handle = heap.new_node(7, links(), None, &mut collector).unwrap();
        let node = heap.node(handle).unwrap();

        assert_eq!(handle.index(), 0);
        assert_eq!(node.variable_id, 7);
        assert_eq!(node.age, 0);
        assert_eq!(node.unique_id, 1);
        assert_eq!(node.halfspace, 0);
        assert_eq!(heap.stats().nodes.used, 1);
        assert_eq!(heap.stats().nodes.unused, 3);
        assert_eq!(heap.stats().nodes.total, 4);
        assert_eq!(heap.stats().blocks.total, 2);
        assert_eq!(heap.lifespan_trace(), &[1]);
    }

    #[test]
    fn full_block_moves_to_next_free_block() {
        let mut heap = BddNodeHeap::new(1, 2).unwrap();
        let mut collector = NoopGarbageCollector;

        let first = heap.new_node(1, links(), None, &mut collector).unwrap();
        let second = heap.new_node(2, links(), None, &mut collector).unwrap();

        assert_ne!(first.block(), second.block());
        assert_eq!(second.index(), 0);
        assert_eq!(heap.stats().nodes.used, 2);
        assert_eq!(heap.stats().nodes.peak, 2);
    }

    #[test]
    fn relocation_preserves_old_debug_identity_without_lifespan_log() {
        let mut heap = BddNodeHeap::new(2, 1).unwrap();
        heap.set_age(4);
        let mut collector = NoopGarbageCollector;
        heap.new_node(1, links(), None, &mut collector).unwrap();
        let trace_len = heap.lifespan_trace().len();
        let old = OldNodeDebug {
            age: 9,
            unique_id: 42,
        };

        let handle = heap
            .new_node(3, links(), Some(old), &mut collector)
            .unwrap();
        let node = heap.node(handle).unwrap();

        assert_eq!(node.age, 9);
        assert_eq!(node.unique_id, 42);
        assert_eq!(heap.lifespan_trace().len(), trace_len);
    }

    #[test]
    fn relocation_without_free_block_is_rejected_before_gc() {
        let mut heap = BddNodeHeap::new(1, 1).unwrap();
        let mut collector = NoopGarbageCollector;
        heap.new_node(1, links(), None, &mut collector).unwrap();

        let error = heap
            .new_node(
                2,
                links(),
                Some(OldNodeDebug {
                    age: 1,
                    unique_id: 1,
                }),
                &mut collector,
            )
            .unwrap_err();

        assert_eq!(error, BddNewNodeError::RelocationWithoutFreeBlock);
    }

    #[test]
    fn full_inuse_heap_invokes_collector_and_grows_by_ratio_policy() {
        let mut heap = BddNodeHeap::new(1, 1).unwrap();
        heap.set_node_ratio(1);
        let mut collector = RecordingCollector::default();

        heap.new_node(1, links(), None, &mut collector).unwrap();
        let second = heap.new_node(2, links(), None, &mut collector).unwrap();

        assert_eq!(collector.calls, 1);
        assert_eq!(heap.node(second).unwrap().variable_id, 2);
        assert!(heap.stats().blocks.total >= 2);
    }

    #[test]
    fn required_allocation_reports_memory_limit() {
        let mut heap = BddNodeHeap::new(2, 1)
            .unwrap()
            .with_allocation_limit_blocks(0);
        let mut collector = NoopGarbageCollector;

        assert_eq!(
            heap.new_node(1, links(), None, &mut collector),
            Err(BddNewNodeError::MemoryLimitExceeded)
        );
    }

    #[test]
    fn halfspace_switch_allocates_from_other_halfspace() {
        let mut heap = BddNodeHeap::new(2, 1).unwrap();
        let mut collector = NoopGarbageCollector;

        heap.new_node(1, links(), None, &mut collector).unwrap();
        heap.set_active_halfspace(1).unwrap();
        let handle = heap.new_node(2, links(), None, &mut collector).unwrap();

        assert_eq!(heap.node(handle).unwrap().halfspace, 1);
        assert_eq!(heap.halfspace(1).unwrap().inuse_blocks().len(), 1);
    }

    #[test]
    fn no_legacy_c_abi_or_source_dependency_tokens_are_present() {
        let source = include_str!("new_node.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
