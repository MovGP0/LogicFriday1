use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MemBlockHandle(usize);

impl MemBlockHandle
{
    pub const fn index(self) -> usize
    {
        self.0
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum MemBlockError
{
    AllocationFailed,
    BlockNotInUse,
    InvalidBlock,
    OutOfBounds,
}

impl fmt::Display for MemBlockError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::AllocationFailed => formatter.write_str("mem_get_block: allocation failed"),
            Self::BlockNotInUse => formatter.write_str("mem_free_block: block not in use"),
            Self::InvalidBlock => formatter.write_str("mem_free_block: invalid block header"),
            Self::OutOfBounds => formatter.write_str("memory block access out of bounds"),
        }
    }
}

impl std::error::Error for MemBlockError
{
}

#[derive(Debug)]
struct MemBlock
{
    bytes: Vec<u8>,
    used: bool,
}

#[derive(Debug)]
pub struct MemBlockAllocator
{
    blocks: BTreeMap<usize, MemBlock>,
    next_handle: usize,
    total_allocation: usize,
}

impl Default for MemBlockAllocator
{
    fn default() -> Self
    {
        Self
        {
            blocks: BTreeMap::new(),
            next_handle: 1,
            total_allocation: 0,
        }
    }
}

impl MemBlockAllocator
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn allocation(&self) -> usize
    {
        self.total_allocation
    }

    pub fn get_block(&mut self, size: usize) -> Result<Option<MemBlockHandle>, MemBlockError>
    {
        if size == 0
        {
            return Ok(None);
        }

        let bytes = vec![0; size];
        let handle = MemBlockHandle(self.next_handle);
        self.next_handle = self
            .next_handle
            .checked_add(1)
            .ok_or(MemBlockError::AllocationFailed)?;
        self.total_allocation = self
            .total_allocation
            .checked_add(size)
            .ok_or(MemBlockError::AllocationFailed)?;
        self.blocks.insert(
            handle.0,
            MemBlock
            {
                bytes,
                used: true,
            },
        );

        Ok(Some(handle))
    }

    pub fn free_block(&mut self, handle: Option<MemBlockHandle>) -> Result<(), MemBlockError>
    {
        let Some(handle) = handle
        else
        {
            return Ok(());
        };

        let block = self
            .blocks
            .get_mut(&handle.0)
            .ok_or(MemBlockError::InvalidBlock)?;

        if !block.used
        {
            return Err(MemBlockError::BlockNotInUse);
        }

        block.used = false;
        Ok(())
    }

    pub fn resize_block(
        &mut self,
        handle: Option<MemBlockHandle>,
        new_size: usize,
    ) -> Result<Option<MemBlockHandle>, MemBlockError>
    {
        let Some(handle) = handle
        else
        {
            return self.get_block(new_size);
        };

        if new_size == 0
        {
            self.free_block(Some(handle))?;
            return Ok(None);
        }

        let block = self
            .blocks
            .get_mut(&handle.0)
            .ok_or(MemBlockError::InvalidBlock)?;

        if !block.used
        {
            return Err(MemBlockError::BlockNotInUse);
        }

        let old_size = block.bytes.len();
        block.bytes.resize(new_size, 0);

        if new_size > old_size
        {
            self.total_allocation = self
                .total_allocation
                .checked_add(new_size - old_size)
                .ok_or(MemBlockError::AllocationFailed)?;
        }

        Ok(Some(handle))
    }

    pub fn block_len(&self, handle: MemBlockHandle) -> Result<usize, MemBlockError>
    {
        let block = self.checked_block(handle)?;
        Ok(block.bytes.len())
    }

    pub fn read_block(&self, handle: MemBlockHandle) -> Result<&[u8], MemBlockError>
    {
        let block = self.checked_block(handle)?;
        Ok(&block.bytes)
    }

    pub fn write_block(&mut self, handle: MemBlockHandle, bytes: &[u8]) -> Result<(), MemBlockError>
    {
        let block = self.checked_block_mut(handle)?;

        if bytes.len() > block.bytes.len()
        {
            return Err(MemBlockError::OutOfBounds);
        }

        block.bytes[..bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    pub fn copy_within(
        &mut self,
        dest: MemBlockHandle,
        dest_offset: usize,
        src: MemBlockHandle,
        src_offset: usize,
        size: usize,
    ) -> Result<(), MemBlockError>
    {
        if size == 0
        {
            return Ok(());
        }

        let src_block = self.checked_block(src)?;
        let src_end = src_offset
            .checked_add(size)
            .ok_or(MemBlockError::OutOfBounds)?;

        if src_end > src_block.bytes.len()
        {
            return Err(MemBlockError::OutOfBounds);
        }

        let copied = src_block.bytes[src_offset..src_end].to_vec();
        let dest_block = self.checked_block_mut(dest)?;
        let dest_end = dest_offset
            .checked_add(size)
            .ok_or(MemBlockError::OutOfBounds)?;

        if dest_end > dest_block.bytes.len()
        {
            return Err(MemBlockError::OutOfBounds);
        }

        dest_block.bytes[dest_offset..dest_end].copy_from_slice(&copied);
        Ok(())
    }

    pub fn zero_block(
        &mut self,
        handle: MemBlockHandle,
        offset: usize,
        size: usize,
    ) -> Result<(), MemBlockError>
    {
        if size == 0
        {
            return Ok(());
        }

        let block = self.checked_block_mut(handle)?;
        let end = offset
            .checked_add(size)
            .ok_or(MemBlockError::OutOfBounds)?;

        if end > block.bytes.len()
        {
            return Err(MemBlockError::OutOfBounds);
        }

        block.bytes[offset..end].fill(0);
        Ok(())
    }

    fn checked_block(&self, handle: MemBlockHandle) -> Result<&MemBlock, MemBlockError>
    {
        let block = self
            .blocks
            .get(&handle.0)
            .ok_or(MemBlockError::InvalidBlock)?;

        if !block.used
        {
            return Err(MemBlockError::BlockNotInUse);
        }

        Ok(block)
    }

    fn checked_block_mut(&mut self, handle: MemBlockHandle) -> Result<&mut MemBlock, MemBlockError>
    {
        let block = self
            .blocks
            .get_mut(&handle.0)
            .ok_or(MemBlockError::InvalidBlock)?;

        if !block.used
        {
            return Err(MemBlockError::BlockNotInUse);
        }

        Ok(block)
    }
}

pub fn mem_copy(dest: &mut [u8], src: &[u8]) -> Result<(), MemBlockError>
{
    if src.len() > dest.len()
    {
        return Err(MemBlockError::OutOfBounds);
    }

    dest[..src.len()].copy_from_slice(src);
    Ok(())
}

pub fn mem_zero(bytes: &mut [u8])
{
    bytes.fill(0);
}

pub fn mem_fatal(message: &str) -> String
{
    format!("Memory management library: error: {message}")
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn zero_size_allocation_returns_no_handle()
    {
        let mut allocator = MemBlockAllocator::new();

        assert_eq!(allocator.get_block(0).unwrap(), None);
        assert_eq!(allocator.allocation(), 0);
    }

    #[test]
    fn allocation_returns_zeroed_storage_and_tracks_bytes()
    {
        let mut allocator = MemBlockAllocator::new();
        let handle = allocator.get_block(4).unwrap().unwrap();

        assert_eq!(allocator.read_block(handle).unwrap(), &[0, 0, 0, 0]);
        assert_eq!(allocator.allocation(), 4);
    }

    #[test]
    fn write_copy_and_zero_preserve_memblock_behavior()
    {
        let mut allocator = MemBlockAllocator::new();
        let source = allocator.get_block(4).unwrap().unwrap();
        let destination = allocator.get_block(5).unwrap().unwrap();

        allocator.write_block(source, &[1, 2, 3, 4]).unwrap();
        allocator.copy_within(destination, 1, source, 1, 3).unwrap();
        assert_eq!(allocator.read_block(destination).unwrap(), &[0, 2, 3, 4, 0]);

        allocator.zero_block(destination, 2, 2).unwrap();
        assert_eq!(allocator.read_block(destination).unwrap(), &[0, 2, 0, 0, 0]);
    }

    #[test]
    fn resize_grows_in_place_and_zero_fills_new_bytes()
    {
        let mut allocator = MemBlockAllocator::new();
        let handle = allocator.get_block(2).unwrap().unwrap();
        allocator.write_block(handle, &[7, 8]).unwrap();

        let resized = allocator.resize_block(Some(handle), 5).unwrap().unwrap();

        assert_eq!(resized, handle);
        assert_eq!(allocator.read_block(handle).unwrap(), &[7, 8, 0, 0, 0]);
        assert_eq!(allocator.allocation(), 5);
    }

    #[test]
    fn resize_shrinks_without_losing_prefix()
    {
        let mut allocator = MemBlockAllocator::new();
        let handle = allocator.get_block(4).unwrap().unwrap();
        allocator.write_block(handle, &[1, 2, 3, 4]).unwrap();

        allocator.resize_block(Some(handle), 2).unwrap();

        assert_eq!(allocator.read_block(handle).unwrap(), &[1, 2]);
        assert_eq!(allocator.allocation(), 4);
    }

    #[test]
    fn resize_to_zero_frees_block()
    {
        let mut allocator = MemBlockAllocator::new();
        let handle = allocator.get_block(4).unwrap().unwrap();

        assert_eq!(allocator.resize_block(Some(handle), 0).unwrap(), None);
        assert_eq!(
            allocator.read_block(handle).unwrap_err(),
            MemBlockError::BlockNotInUse
        );
    }

    #[test]
    fn double_free_reports_block_not_in_use()
    {
        let mut allocator = MemBlockAllocator::new();
        let handle = allocator.get_block(4).unwrap().unwrap();

        allocator.free_block(Some(handle)).unwrap();

        assert_eq!(
            allocator.free_block(Some(handle)).unwrap_err(),
            MemBlockError::BlockNotInUse
        );
    }

    #[test]
    fn invalid_handle_reports_invalid_block()
    {
        let mut allocator = MemBlockAllocator::new();

        assert_eq!(
            allocator.free_block(Some(MemBlockHandle(42))).unwrap_err(),
            MemBlockError::InvalidBlock
        );
    }

    #[test]
    fn standalone_copy_and_zero_helpers_match_legacy_macros()
    {
        let mut destination = [0, 0, 0, 0];
        let source = [9, 8, 7];

        mem_copy(&mut destination, &source).unwrap();
        assert_eq!(destination, [9, 8, 7, 0]);

        mem_zero(&mut destination[1..3]);
        assert_eq!(destination, [9, 0, 0, 0]);
    }

    #[test]
    fn fatal_message_matches_legacy_prefix()
    {
        assert_eq!(
            mem_fatal("mem_get_block: allocation failed"),
            "Memory management library: error: mem_get_block: allocation failed"
        );
    }
}
