//! Native Rust external pointer map for the UCB BDD package.
//!
//! The legacy implementation stores `bdd_t` entries in fixed-size blocks and
//! uses a roving pointer to reuse freed slots. This module keeps that behavior
//! in Rust-owned handles instead of exposing raw C pointers.

use std::fmt;

pub const DEFAULT_EXTERNAL_POINTER_BLOCK_SIZE: usize = 128;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExternalPointerStats {
    pub used: usize,
    pub unused: usize,
    pub total: usize,
    pub blocks: usize,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ExternalPointerHandle {
    block: usize,
    index: usize,
}

impl ExternalPointerHandle {
    pub fn block(self) -> usize {
        self.block
    }

    pub fn index(self) -> usize {
        self.index
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExternalPointerError {
    EmptyBlockSize,
    UnknownHandle(ExternalPointerHandle),
}

impl fmt::Display for ExternalPointerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBlockSize => {
                formatter.write_str("external pointer block size must be non-zero")
            }
            Self::UnknownHandle(handle) => {
                write!(
                    formatter,
                    "external pointer handle ({}, {}) is not allocated by this manager",
                    handle.block, handle.index
                )
            }
        }
    }
}

impl std::error::Error for ExternalPointerError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalPointer<N> {
    node: Option<N>,
    origin: Option<&'static str>,
}

impl<N> ExternalPointer<N> {
    pub fn node(&self) -> Option<&N> {
        self.node.as_ref()
    }

    pub fn node_mut(&mut self) -> Option<&mut N> {
        self.node.as_mut()
    }

    pub fn origin(&self) -> Option<&'static str> {
        self.origin
    }
}

#[derive(Clone, Debug)]
struct Slot<N> {
    entry: Option<ExternalPointer<N>>,
}

impl<N> Slot<N> {
    fn free() -> Self {
        Self { entry: None }
    }

    fn is_free(&self) -> bool {
        self.entry.is_none()
    }
}

#[derive(Clone, Debug)]
struct Block<N> {
    slots: Vec<Slot<N>>,
}

impl<N> Block<N> {
    fn new(size: usize) -> Self {
        let slots = (0..size).map(|_| Slot::free()).collect();

        Self { slots }
    }

    fn len(&self) -> usize {
        self.slots.len()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct RovingPointer {
    block: usize,
    index: usize,
    initialized: bool,
}

#[derive(Clone, Debug)]
pub struct ExternalPointerManager<N> {
    blocks: Vec<Block<N>>,
    block_size: usize,
    free: usize,
    nmap: usize,
    pointer: RovingPointer,
    stats: ExternalPointerStats,
}

impl<N> Default for ExternalPointerManager<N> {
    fn default() -> Self {
        Self::new(DEFAULT_EXTERNAL_POINTER_BLOCK_SIZE)
            .expect("default external pointer block size is non-zero")
    }
}

impl<N> ExternalPointerManager<N> {
    pub fn new(block_size: usize) -> Result<Self, ExternalPointerError> {
        if block_size == 0 {
            return Err(ExternalPointerError::EmptyBlockSize);
        }

        Ok(Self {
            blocks: Vec::new(),
            block_size,
            free: 0,
            nmap: 0,
            pointer: RovingPointer::default(),
            stats: ExternalPointerStats::default(),
        })
    }

    pub fn make_external_pointer(
        &mut self,
        node: Option<N>,
        origin: Option<&'static str>,
    ) -> ExternalPointerHandle {
        if self.free == 0 {
            self.new_block();
        }

        if !self.pointer.initialized {
            self.pointer = RovingPointer {
                block: 0,
                index: 0,
                initialized: true,
            };
        }

        for _ in 0..self.nmap {
            self.normalize_roving_pointer();

            let block = self.pointer.block;
            let index = self.pointer.index;

            if self.blocks[block].slots[index].is_free() {
                self.pointer.index += 1;
                self.blocks[block].slots[index].entry = Some(ExternalPointer { node, origin });
                self.free -= 1;
                self.stats.used += 1;
                self.stats.unused -= 1;

                return ExternalPointerHandle { block, index };
            }

            self.pointer.index += 1;
        }

        self.new_block();
        self.make_external_pointer(node, origin)
    }

    pub fn destroy_external_pointer(
        &mut self,
        handle: ExternalPointerHandle,
    ) -> Result<bool, ExternalPointerError> {
        let slot = self
            .blocks
            .get_mut(handle.block)
            .and_then(|block| block.slots.get_mut(handle.index))
            .ok_or(ExternalPointerError::UnknownHandle(handle))?;

        if slot.entry.is_none() {
            return Ok(false);
        }

        slot.entry = None;
        self.free += 1;
        self.stats.used -= 1;
        self.stats.unused += 1;

        Ok(true)
    }

    pub fn get(&self, handle: ExternalPointerHandle) -> Option<&ExternalPointer<N>> {
        self.blocks
            .get(handle.block)
            .and_then(|block| block.slots.get(handle.index))
            .and_then(|slot| slot.entry.as_ref())
    }

    pub fn get_mut(&mut self, handle: ExternalPointerHandle) -> Option<&mut ExternalPointer<N>> {
        self.blocks
            .get_mut(handle.block)
            .and_then(|block| block.slots.get_mut(handle.index))
            .and_then(|slot| slot.entry.as_mut())
    }

    pub fn index_of_external_pointer(
        &self,
        handle: ExternalPointerHandle,
    ) -> Result<usize, ExternalPointerError> {
        self.blocks
            .get(handle.block)
            .and_then(|block| block.slots.get(handle.index))
            .ok_or(ExternalPointerError::UnknownHandle(handle))?;

        Ok(handle.index)
    }

    pub fn stats(&self) -> ExternalPointerStats {
        self.stats
    }

    pub fn free_count(&self) -> usize {
        self.free
    }

    pub fn mapped_count(&self) -> usize {
        self.nmap
    }

    fn new_block(&mut self) {
        let block_index = self.blocks.len();
        self.blocks.push(Block::new(self.block_size));

        self.free += self.block_size;
        self.nmap += self.block_size;
        self.pointer = RovingPointer {
            block: block_index,
            index: 0,
            initialized: true,
        };
        self.stats.blocks += 1;
        self.stats.unused += self.block_size;
        self.stats.total += self.block_size;
    }

    fn normalize_roving_pointer(&mut self) {
        if self.pointer.index < self.blocks[self.pointer.block].len() {
            return;
        }

        self.pointer.block += 1;
        self.pointer.index = 0;

        if self.pointer.block == self.blocks.len() {
            self.pointer.block = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_rejects_empty_block_size() {
        let error = ExternalPointerManager::<u32>::new(0).unwrap_err();

        assert_eq!(error, ExternalPointerError::EmptyBlockSize);
    }

    #[test]
    fn first_allocation_creates_a_block_and_stores_payload() {
        let mut manager = ExternalPointerManager::new(2).unwrap();

        let handle = manager.make_external_pointer(Some(42), Some("test"));

        assert_eq!(handle, ExternalPointerHandle { block: 0, index: 0 });
        assert_eq!(manager.free_count(), 1);
        assert_eq!(
            manager.stats(),
            ExternalPointerStats {
                used: 1,
                unused: 1,
                total: 2,
                blocks: 1
            }
        );
        assert_eq!(manager.get(handle).unwrap().node(), Some(&42));
        assert_eq!(manager.get(handle).unwrap().origin(), Some("test"));
    }

    #[test]
    fn allocation_uses_roving_pointer_before_reusing_earlier_free_slot() {
        let mut manager = ExternalPointerManager::new(3).unwrap();

        let first = manager.make_external_pointer(Some(1), None);
        let second = manager.make_external_pointer(Some(2), None);

        assert!(manager.destroy_external_pointer(first).unwrap());

        let third = manager.make_external_pointer(Some(3), None);

        assert_eq!(second.index(), 1);
        assert_eq!(third.index(), 2);
        assert_eq!(manager.get(first), None);
        assert_eq!(manager.get(third).unwrap().node(), Some(&3));
    }

    #[test]
    fn allocation_wraps_and_reuses_freed_slot() {
        let mut manager = ExternalPointerManager::new(2).unwrap();

        let first = manager.make_external_pointer(Some(1), None);
        let second = manager.make_external_pointer(Some(2), None);

        assert!(manager.destroy_external_pointer(first).unwrap());

        let reused = manager.make_external_pointer(Some(3), None);

        assert_eq!(reused, first);
        assert_eq!(manager.get(reused).unwrap().node(), Some(&3));
        assert_eq!(manager.get(second).unwrap().node(), Some(&2));
    }

    #[test]
    fn full_map_allocates_new_block_and_starts_there() {
        let mut manager = ExternalPointerManager::new(1).unwrap();

        let first = manager.make_external_pointer(Some(1), None);
        let second = manager.make_external_pointer(Some(2), None);

        assert_eq!(first, ExternalPointerHandle { block: 0, index: 0 });
        assert_eq!(second, ExternalPointerHandle { block: 1, index: 0 });
        assert_eq!(
            manager.stats(),
            ExternalPointerStats {
                used: 2,
                unused: 0,
                total: 2,
                blocks: 2
            }
        );
    }

    #[test]
    fn destroy_is_idempotent_for_allocated_slots() {
        let mut manager = ExternalPointerManager::new(2).unwrap();
        let handle = manager.make_external_pointer(Some(1), None);

        assert!(manager.destroy_external_pointer(handle).unwrap());
        assert!(!manager.destroy_external_pointer(handle).unwrap());
        assert_eq!(
            manager.stats(),
            ExternalPointerStats {
                used: 0,
                unused: 2,
                total: 2,
                blocks: 1
            }
        );
    }

    #[test]
    fn unknown_handle_is_rejected() {
        let mut manager = ExternalPointerManager::<u32>::new(2).unwrap();
        let handle = ExternalPointerHandle { block: 4, index: 0 };

        assert_eq!(
            manager.destroy_external_pointer(handle),
            Err(ExternalPointerError::UnknownHandle(handle))
        );
        assert_eq!(
            manager.index_of_external_pointer(handle),
            Err(ExternalPointerError::UnknownHandle(handle))
        );
    }

    #[test]
    fn mutable_access_updates_stored_node() {
        let mut manager = ExternalPointerManager::new(2).unwrap();
        let handle = manager.make_external_pointer(Some(String::from("before")), None);

        *manager.get_mut(handle).unwrap().node_mut().unwrap() = String::from("after");

        assert_eq!(
            manager.get(handle).unwrap().node(),
            Some(&String::from("after"))
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_dependency_metadata_are_present() {
        let source = include_str!("external_ptr.rs");

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
