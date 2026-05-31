//! Native Rust fixed-size record manager for the CMU BDD memory package.
//!
//! The legacy `memrec.c` allocator carves aligned records out of lazily
//! allocated blocks and links free records through the record storage itself.
//! This module keeps that behavior with Rust-owned blocks and opaque handles.

use std::fmt;

pub const ALLOCATION_ALIGNMENT: usize = 8;
pub const DEFAULT_ALLOCATION_BYTES: usize = 4096;

const LIST_HEADER_BYTES: usize = std::mem::size_of::<usize>();

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RecordHandle {
    block: usize,
    index: usize,
}

impl RecordHandle {
    pub fn block(self) -> usize {
        self.block
    }

    pub fn index(self) -> usize {
        self.index
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecordManagerStats {
    pub record_size: usize,
    pub records_per_block: usize,
    pub block_count: usize,
    pub allocated_records: usize,
    pub free_records: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecordManagerError {
    AllocationTooSmall {
        allocation_bytes: usize,
    },
    RecordTooLarge {
        record_size: usize,
        maximum_size: usize,
    },
    UnknownRecord(RecordHandle),
    RecordAlreadyFree(RecordHandle),
}

impl fmt::Display for RecordManagerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllocationTooSmall { allocation_bytes } => {
                write!(
                    formatter,
                    "record allocation block of {allocation_bytes} bytes is too small"
                )
            }
            Self::RecordTooLarge {
                record_size,
                maximum_size,
            } => {
                write!(
                    formatter,
                    "record size {record_size} exceeds maximum record size {maximum_size}"
                )
            }
            Self::UnknownRecord(handle) => {
                write!(
                    formatter,
                    "record handle ({}, {}) does not belong to this manager",
                    handle.block, handle.index
                )
            }
            Self::RecordAlreadyFree(handle) => {
                write!(
                    formatter,
                    "record handle ({}, {}) is already free",
                    handle.block, handle.index
                )
            }
        }
    }
}

impl std::error::Error for RecordManagerError {}

#[derive(Clone, Debug)]
struct RecordSlot {
    storage: Vec<u8>,
    allocated: bool,
}

impl RecordSlot {
    fn new(size: usize) -> Self {
        Self {
            storage: vec![0; size],
            allocated: false,
        }
    }
}

#[derive(Clone, Debug)]
struct RecordBlock {
    slots: Vec<RecordSlot>,
}

impl RecordBlock {
    fn new(record_size: usize, records_per_block: usize) -> Self {
        let slots = (0..records_per_block)
            .map(|_| RecordSlot::new(record_size))
            .collect();

        Self { slots }
    }
}

#[derive(Clone, Debug)]
pub struct RecordManager {
    record_size: usize,
    records_per_block: usize,
    blocks: Vec<RecordBlock>,
    free: Vec<RecordHandle>,
    allocated_records: usize,
}

impl RecordManager {
    pub fn new(requested_record_size: usize) -> Result<Self, RecordManagerError> {
        Self::with_allocation_bytes(requested_record_size, DEFAULT_ALLOCATION_BYTES)
    }

    pub fn with_allocation_bytes(
        requested_record_size: usize,
        allocation_bytes: usize,
    ) -> Result<Self, RecordManagerError> {
        let header_size = roundup(LIST_HEADER_BYTES);

        if allocation_bytes <= header_size {
            return Err(RecordManagerError::AllocationTooSmall { allocation_bytes });
        }

        let record_size = roundup(requested_record_size.max(LIST_HEADER_BYTES));
        let maximum_size = allocation_bytes - header_size;

        if record_size > maximum_size {
            return Err(RecordManagerError::RecordTooLarge {
                record_size,
                maximum_size,
            });
        }

        Ok(Self {
            record_size,
            records_per_block: maximum_size / record_size,
            blocks: Vec::new(),
            free: Vec::new(),
            allocated_records: 0,
        })
    }

    pub fn allocate(&mut self) -> RecordHandle {
        if self.free.is_empty() {
            self.allocate_block();
        }

        let handle = self
            .free
            .pop()
            .expect("a newly allocated block supplies at least one record");
        let slot = self
            .slot_mut(handle)
            .expect("free list only contains manager-owned records");

        slot.allocated = true;
        self.allocated_records += 1;

        handle
    }

    pub fn free(&mut self, handle: RecordHandle) -> Result<(), RecordManagerError> {
        let slot = self.slot_mut(handle)?;

        if !slot.allocated {
            return Err(RecordManagerError::RecordAlreadyFree(handle));
        }

        slot.allocated = false;
        self.allocated_records -= 1;
        self.free.push(handle);

        Ok(())
    }

    pub fn get(&self, handle: RecordHandle) -> Result<&[u8], RecordManagerError> {
        let slot = self.slot(handle)?;

        if !slot.allocated {
            return Err(RecordManagerError::RecordAlreadyFree(handle));
        }

        Ok(&slot.storage)
    }

    pub fn get_mut(&mut self, handle: RecordHandle) -> Result<&mut [u8], RecordManagerError> {
        let slot = self.slot_mut(handle)?;

        if !slot.allocated {
            return Err(RecordManagerError::RecordAlreadyFree(handle));
        }

        Ok(&mut slot.storage)
    }

    pub fn clear(&mut self, handle: RecordHandle) -> Result<(), RecordManagerError> {
        self.get_mut(handle)?.fill(0);

        Ok(())
    }

    pub fn release_all(self) {}

    pub fn record_size(&self) -> usize {
        self.record_size
    }

    pub fn records_per_block(&self) -> usize {
        self.records_per_block
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    pub fn stats(&self) -> RecordManagerStats {
        RecordManagerStats {
            record_size: self.record_size,
            records_per_block: self.records_per_block,
            block_count: self.block_count(),
            allocated_records: self.allocated_records,
            free_records: self.free.len(),
        }
    }

    fn allocate_block(&mut self) {
        let block_index = self.blocks.len();

        self.blocks
            .push(RecordBlock::new(self.record_size, self.records_per_block));

        for index in (0..self.records_per_block).rev() {
            self.free.push(RecordHandle {
                block: block_index,
                index,
            });
        }
    }

    fn slot(&self, handle: RecordHandle) -> Result<&RecordSlot, RecordManagerError> {
        self.blocks
            .get(handle.block)
            .and_then(|block| block.slots.get(handle.index))
            .ok_or(RecordManagerError::UnknownRecord(handle))
    }

    fn slot_mut(&mut self, handle: RecordHandle) -> Result<&mut RecordSlot, RecordManagerError> {
        self.blocks
            .get_mut(handle.block)
            .and_then(|block| block.slots.get_mut(handle.index))
            .ok_or(RecordManagerError::UnknownRecord(handle))
    }
}

pub fn roundup(size: usize) -> usize {
    size.div_ceil(ALLOCATION_ALIGNMENT) * ALLOCATION_ALIGNMENT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_record_size_and_applies_pointer_sized_minimum() {
        let manager = RecordManager::with_allocation_bytes(1, 72).unwrap();

        assert_eq!(manager.record_size(), roundup(LIST_HEADER_BYTES));
        assert_eq!(manager.records_per_block(), 8);
    }

    #[test]
    fn rejects_blocks_that_cannot_hold_records_after_header() {
        let error =
            RecordManager::with_allocation_bytes(8, roundup(LIST_HEADER_BYTES)).unwrap_err();

        assert_eq!(
            error,
            RecordManagerError::AllocationTooSmall {
                allocation_bytes: roundup(LIST_HEADER_BYTES)
            }
        );
    }

    #[test]
    fn rejects_records_larger_than_available_block_payload() {
        let error = RecordManager::with_allocation_bytes(65, 72).unwrap_err();

        assert_eq!(
            error,
            RecordManagerError::RecordTooLarge {
                record_size: 72,
                maximum_size: 64
            }
        );
    }

    #[test]
    fn first_allocation_lazily_creates_block_and_returns_first_record() {
        let mut manager = RecordManager::with_allocation_bytes(8, 40).unwrap();

        let handle = manager.allocate();

        assert_eq!(handle, RecordHandle { block: 0, index: 0 });
        assert_eq!(
            manager.stats(),
            RecordManagerStats {
                record_size: 8,
                records_per_block: 4,
                block_count: 1,
                allocated_records: 1,
                free_records: 3
            }
        );
    }

    #[test]
    fn allocations_follow_the_carved_block_order() {
        let mut manager = RecordManager::with_allocation_bytes(8, 32).unwrap();

        let first = manager.allocate();
        let second = manager.allocate();
        let third = manager.allocate();

        assert_eq!(first.index(), 0);
        assert_eq!(second.index(), 1);
        assert_eq!(third.index(), 2);
    }

    #[test]
    fn free_reuses_records_in_lifo_order() {
        let mut manager = RecordManager::with_allocation_bytes(8, 32).unwrap();
        let first = manager.allocate();
        let second = manager.allocate();

        manager.free(first).unwrap();
        manager.free(second).unwrap();

        assert_eq!(manager.allocate(), second);
        assert_eq!(manager.allocate(), first);
    }

    #[test]
    fn storage_can_be_mutated_and_cleared_by_handle() {
        let mut manager = RecordManager::with_allocation_bytes(4, 32).unwrap();
        let handle = manager.allocate();

        manager.get_mut(handle).unwrap()[0..4].copy_from_slice(&[1, 2, 3, 4]);

        assert_eq!(&manager.get(handle).unwrap()[0..4], &[1, 2, 3, 4]);

        manager.clear(handle).unwrap();

        assert_eq!(&manager.get(handle).unwrap()[0..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn full_block_allocates_another_block() {
        let mut manager = RecordManager::with_allocation_bytes(8, 16).unwrap();

        let first = manager.allocate();
        let second = manager.allocate();

        assert_eq!(first, RecordHandle { block: 0, index: 0 });
        assert_eq!(second, RecordHandle { block: 1, index: 0 });
        assert_eq!(manager.block_count(), 2);
    }

    #[test]
    fn rejects_unknown_and_already_free_records() {
        let mut manager = RecordManager::with_allocation_bytes(8, 32).unwrap();
        let handle = manager.allocate();
        let unknown = RecordHandle {
            block: 99,
            index: 0,
        };

        manager.free(handle).unwrap();

        assert_eq!(
            manager.free(handle),
            Err(RecordManagerError::RecordAlreadyFree(handle))
        );
        assert_eq!(
            manager.get(handle),
            Err(RecordManagerError::RecordAlreadyFree(handle))
        );
        assert_eq!(
            manager.free(unknown),
            Err(RecordManagerError::UnknownRecord(unknown))
        );
    }

    #[test]
    fn default_manager_uses_legacy_sized_blocks() {
        let manager = RecordManager::new(24).unwrap();

        assert_eq!(manager.record_size(), 24);
        assert_eq!(
            manager.records_per_block(),
            (DEFAULT_ALLOCATION_BYTES - roundup(LIST_HEADER_BYTES)) / 24
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_dependency_metadata_are_present() {
        let source = include_str!("memrec.rs");

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
