//! Native Rust heap assertion model for the SIS UCB BDD package.
//!
//! The legacy `assert_heap.c` walk checks every BDD pointer reachable from the
//! manager heap, safe frames, external references, and caches. This module keeps
//! that diagnostic behavior in owned Rust data and returns typed errors instead
//! of relying on C assertions.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddNodeHandle {
    block: usize,
    index: usize,
    complemented: bool,
}

impl BddNodeHandle {
    pub fn regular(block: usize, index: usize) -> Self {
        Self {
            block,
            index,
            complemented: false,
        }
    }

    pub fn complemented(block: usize, index: usize) -> Self {
        Self {
            block,
            index,
            complemented: true,
        }
    }

    pub fn block(self) -> usize {
        self.block
    }

    pub fn index(self) -> usize {
        self.index
    }

    pub fn is_complemented(self) -> bool {
        self.complemented
    }

    pub fn regularized(self) -> Self {
        Self {
            block: self.block,
            index: self.index,
            complemented: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddHeapNode {
    pub then_child: Option<BddNodeHandle>,
    pub else_child: Option<BddNodeHandle>,
    pub halfspace: usize,
    pub broken_heart: bool,
}

impl BddHeapNode {
    pub fn new(halfspace: usize) -> Self {
        Self {
            then_child: None,
            else_child: None,
            halfspace,
            broken_heart: false,
        }
    }

    pub fn with_children(
        mut self,
        then_child: Option<BddNodeHandle>,
        else_child: Option<BddNodeHandle>,
    ) -> Self {
        self.then_child = then_child;
        self.else_child = else_child;
        self
    }

    pub fn as_broken_heart(mut self) -> Self {
        self.broken_heart = true;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddNodeBlock {
    nodes: Vec<BddHeapNode>,
    used: usize,
}

impl BddNodeBlock {
    pub fn new(nodes: Vec<BddHeapNode>) -> Self {
        let used = nodes.len();
        Self { nodes, used }
    }

    pub fn with_used(mut self, used: usize) -> Self {
        self.used = used.min(self.nodes.len());
        self
    }

    pub fn used_nodes(&self) -> &[BddHeapNode] {
        &self.nodes[..self.used]
    }

    pub fn node(&self, index: usize) -> Option<&BddHeapNode> {
        if index < self.used {
            self.nodes.get(index)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddSafeNode {
    pub arg: Option<BddNodeHandle>,
    pub node: Option<BddNodeHandle>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddSafeFrame {
    pub nodes: Vec<BddSafeNode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddExternalRef {
    pub free: bool,
    pub node: Option<BddNodeHandle>,
}

impl BddExternalRef {
    pub fn free() -> Self {
        Self {
            free: true,
            node: None,
        }
    }

    pub fn used(node: BddNodeHandle) -> Self {
        Self {
            free: false,
            node: Some(node),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddExternalBlock {
    pub refs: Vec<BddExternalRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddIteCacheEntry {
    pub f: BddNodeHandle,
    pub g: BddNodeHandle,
    pub h: BddNodeHandle,
    pub data: BddNodeHandle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddConstCacheEntry {
    pub f: BddNodeHandle,
    pub g: BddNodeHandle,
    pub h: BddNodeHandle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddAdhocCacheEntry {
    pub f: BddNodeHandle,
    pub g: BddNodeHandle,
    pub data: Option<BddNodeHandle>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddHeapCaches {
    pub ite: Vec<Option<BddIteCacheEntry>>,
    pub constant: Vec<Option<BddConstCacheEntry>>,
    pub adhoc: Vec<BddAdhocCacheEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddHeapManager {
    active_halfspace: usize,
    one: Option<BddNodeHandle>,
    blocks: Vec<BddNodeBlock>,
    safe_frames: Vec<BddSafeFrame>,
    external_blocks: Vec<BddExternalBlock>,
    pub caches: BddHeapCaches,
}

impl BddHeapManager {
    pub fn new(active_halfspace: usize) -> Self {
        Self {
            active_halfspace,
            one: None,
            blocks: Vec::new(),
            safe_frames: Vec::new(),
            external_blocks: Vec::new(),
            caches: BddHeapCaches::default(),
        }
    }

    pub fn active_halfspace(&self) -> usize {
        self.active_halfspace
    }

    pub fn set_one(&mut self, node: Option<BddNodeHandle>) {
        self.one = node;
    }

    pub fn add_block(&mut self, block: BddNodeBlock) -> usize {
        let index = self.blocks.len();
        self.blocks.push(block);
        index
    }

    pub fn add_safe_frame(&mut self, frame: BddSafeFrame) {
        self.safe_frames.push(frame);
    }

    pub fn add_external_block(&mut self, block: BddExternalBlock) {
        self.external_blocks.push(block);
    }

    pub fn node(&self, handle: BddNodeHandle) -> Option<&BddHeapNode> {
        let regular = handle.regularized();
        self.blocks
            .get(regular.block)
            .and_then(|block| block.node(regular.index))
    }

    pub fn assert_heap_correct(&self) -> Result<(), BddHeapAssertionError> {
        assert_pointer_okay(self, self.one, BddHeapReference::One)?;

        for (block_index, block) in self.blocks.iter().enumerate() {
            for (node_index, node) in block.used_nodes().iter().enumerate() {
                let reference = BddHeapReference::NodeChild {
                    block: block_index,
                    index: node_index,
                };
                assert_pointer_okay(self, node.then_child, reference)?;
                assert_pointer_okay(self, node.else_child, reference)?;
            }
        }

        for (frame_index, frame) in self.safe_frames.iter().enumerate() {
            for (node_index, safe_node) in frame.nodes.iter().enumerate() {
                let reference = BddHeapReference::SafeFrame {
                    frame: frame_index,
                    index: node_index,
                };
                assert_pointer_okay(self, safe_node.arg, reference)?;
                assert_pointer_okay(self, safe_node.node, reference)?;
            }
        }

        for (block_index, block) in self.external_blocks.iter().enumerate() {
            for (ref_index, external_ref) in block.refs.iter().enumerate() {
                if !external_ref.free {
                    assert_pointer_okay(
                        self,
                        external_ref.node,
                        BddHeapReference::External {
                            block: block_index,
                            index: ref_index,
                        },
                    )?;
                }
            }
        }

        for (bucket, entry) in self.caches.ite.iter().enumerate() {
            if let Some(entry) = entry {
                let reference = BddHeapReference::IteCache { bucket };
                assert_pointer_okay(self, Some(entry.f), reference)?;
                assert_pointer_okay(self, Some(entry.g), reference)?;
                assert_pointer_okay(self, Some(entry.h), reference)?;
                assert_pointer_okay(self, Some(entry.data), reference)?;
            }
        }

        for (bucket, entry) in self.caches.constant.iter().enumerate() {
            if let Some(entry) = entry {
                let reference = BddHeapReference::ConstCache { bucket };
                assert_pointer_okay(self, Some(entry.f), reference)?;
                assert_pointer_okay(self, Some(entry.g), reference)?;
                assert_pointer_okay(self, Some(entry.h), reference)?;
            }
        }

        for (index, entry) in self.caches.adhoc.iter().enumerate() {
            let reference = BddHeapReference::AdhocCache { index };
            assert_pointer_okay(self, Some(entry.f), reference)?;
            assert_pointer_okay(self, Some(entry.g), reference)?;
            assert_pointer_okay(self, entry.data, reference)?;
        }

        Ok(())
    }
}

pub fn bdd_assert_heap_correct(manager: &BddHeapManager) -> Result<(), BddHeapAssertionError> {
    manager.assert_heap_correct()
}

fn assert_pointer_okay(
    manager: &BddHeapManager,
    pointer: Option<BddNodeHandle>,
    reference: BddHeapReference,
) -> Result<(), BddHeapAssertionError> {
    let Some(pointer) = pointer else {
        return Ok(());
    };

    let regular = pointer.regularized();
    let node = manager
        .node(regular)
        .ok_or(BddHeapAssertionError::MissingNode {
            pointer: regular,
            reference,
        })?;

    if node.halfspace != manager.active_halfspace {
        return Err(BddHeapAssertionError::WrongHalfspace {
            pointer: regular,
            expected: manager.active_halfspace,
            actual: node.halfspace,
            reference,
        });
    }

    if node.broken_heart {
        return Err(BddHeapAssertionError::BrokenHeart {
            pointer: regular,
            reference,
        });
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddHeapReference {
    One,
    NodeChild { block: usize, index: usize },
    SafeFrame { frame: usize, index: usize },
    External { block: usize, index: usize },
    IteCache { bucket: usize },
    ConstCache { bucket: usize },
    AdhocCache { index: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddHeapAssertionError {
    MissingNode {
        pointer: BddNodeHandle,
        reference: BddHeapReference,
    },
    WrongHalfspace {
        pointer: BddNodeHandle,
        expected: usize,
        actual: usize,
        reference: BddHeapReference,
    },
    BrokenHeart {
        pointer: BddNodeHandle,
        reference: BddHeapReference,
    },
}

impl fmt::Display for BddHeapAssertionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode { pointer, reference } => {
                write!(
                    formatter,
                    "BDD heap reference {reference:?} points to missing node {}:{}",
                    pointer.block(),
                    pointer.index()
                )
            }
            Self::WrongHalfspace {
                pointer,
                expected,
                actual,
                reference,
            } => {
                write!(
                    formatter,
                    "BDD heap reference {reference:?} points to node {}:{} in halfspace {actual}, expected {expected}",
                    pointer.block(),
                    pointer.index()
                )
            }
            Self::BrokenHeart { pointer, reference } => {
                write!(
                    formatter,
                    "BDD heap reference {reference:?} points to broken-heart node {}:{}",
                    pointer.block(),
                    pointer.index()
                )
            }
        }
    }
}

impl Error for BddHeapAssertionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn live_node() -> BddHeapNode {
        BddHeapNode::new(0)
    }

    fn handle(index: usize) -> BddNodeHandle {
        BddNodeHandle::regular(0, index)
    }

    #[test]
    fn accepts_heap_with_valid_roots_refs_and_caches() {
        let mut manager = BddHeapManager::new(0);
        manager.add_block(BddNodeBlock::new(vec![
            live_node().with_children(Some(handle(1)), Some(BddNodeHandle::complemented(0, 2))),
            live_node(),
            live_node(),
        ]));
        manager.set_one(Some(BddNodeHandle::complemented(0, 0)));
        manager.add_safe_frame(BddSafeFrame {
            nodes: vec![BddSafeNode {
                arg: Some(handle(1)),
                node: Some(handle(2)),
            }],
        });
        manager.add_external_block(BddExternalBlock {
            refs: vec![BddExternalRef::free(), BddExternalRef::used(handle(0))],
        });
        manager.caches.ite.push(Some(BddIteCacheEntry {
            f: handle(0),
            g: handle(1),
            h: handle(2),
            data: BddNodeHandle::complemented(0, 1),
        }));
        manager.caches.constant.push(Some(BddConstCacheEntry {
            f: handle(0),
            g: handle(1),
            h: handle(2),
        }));
        manager.caches.adhoc.push(BddAdhocCacheEntry {
            f: handle(0),
            g: handle(1),
            data: Some(handle(2)),
        });

        assert_eq!(bdd_assert_heap_correct(&manager), Ok(()));
    }

    #[test]
    fn missing_one_node_is_reported() {
        let mut manager = BddHeapManager::new(0);
        manager.set_one(Some(handle(0)));

        assert_eq!(
            bdd_assert_heap_correct(&manager),
            Err(BddHeapAssertionError::MissingNode {
                pointer: handle(0),
                reference: BddHeapReference::One,
            })
        );
    }

    #[test]
    fn ignores_unused_block_slots_like_legacy_used_count() {
        let mut manager = BddHeapManager::new(0);
        manager.add_block(
            BddNodeBlock::new(vec![
                live_node(),
                live_node().with_children(Some(BddNodeHandle::regular(0, 99)), None),
            ])
            .with_used(1),
        );
        manager.set_one(Some(handle(0)));

        assert_eq!(manager.assert_heap_correct(), Ok(()));
    }

    #[test]
    fn detects_wrong_halfspace_after_regularizing_complemented_pointer() {
        let mut manager = BddHeapManager::new(0);
        manager.add_block(BddNodeBlock::new(vec![BddHeapNode::new(1)]));
        manager.set_one(Some(BddNodeHandle::complemented(0, 0)));

        assert_eq!(
            manager.assert_heap_correct(),
            Err(BddHeapAssertionError::WrongHalfspace {
                pointer: handle(0),
                expected: 0,
                actual: 1,
                reference: BddHeapReference::One,
            })
        );
    }

    #[test]
    fn detects_broken_heart_node_from_internal_child() {
        let mut manager = BddHeapManager::new(0);
        manager.add_block(BddNodeBlock::new(vec![
            live_node().with_children(Some(handle(1)), None),
            live_node().as_broken_heart(),
        ]));

        assert_eq!(
            manager.assert_heap_correct(),
            Err(BddHeapAssertionError::BrokenHeart {
                pointer: handle(1),
                reference: BddHeapReference::NodeChild { block: 0, index: 0 },
            })
        );
    }

    #[test]
    fn free_external_refs_and_empty_safe_args_are_ignored() {
        let mut manager = BddHeapManager::new(0);
        manager.add_block(BddNodeBlock::new(vec![live_node()]));
        manager.add_safe_frame(BddSafeFrame {
            nodes: vec![BddSafeNode {
                arg: None,
                node: None,
            }],
        });
        manager.add_external_block(BddExternalBlock {
            refs: vec![BddExternalRef {
                free: true,
                node: Some(BddNodeHandle::regular(50, 50)),
            }],
        });

        assert_eq!(manager.assert_heap_correct(), Ok(()));
    }

    #[test]
    fn ite_cache_data_is_validated() {
        let mut manager = BddHeapManager::new(0);
        manager.add_block(BddNodeBlock::new(vec![live_node()]));
        manager.caches.ite.push(Some(BddIteCacheEntry {
            f: handle(0),
            g: handle(0),
            h: handle(0),
            data: BddNodeHandle::regular(0, 9),
        }));

        assert_eq!(
            manager.assert_heap_correct(),
            Err(BddHeapAssertionError::MissingNode {
                pointer: BddNodeHandle::regular(0, 9),
                reference: BddHeapReference::IteCache { bucket: 0 },
            })
        );
    }

    #[test]
    fn const_cache_has_no_data_pointer_to_validate() {
        let mut manager = BddHeapManager::new(0);
        manager.add_block(BddNodeBlock::new(vec![live_node()]));
        manager.caches.constant.push(Some(BddConstCacheEntry {
            f: handle(0),
            g: handle(0),
            h: handle(0),
        }));

        assert_eq!(manager.assert_heap_correct(), Ok(()));
    }

    #[test]
    fn no_legacy_c_abi_or_dependency_metadata_tokens_are_present() {
        let source = include_str!("assert_heap.rs");

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
