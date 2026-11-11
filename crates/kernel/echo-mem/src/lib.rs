#![no_std]
#![forbid(unsafe_code)]

//! 静态固定块内存池（Q2）。
//!
//! 单核、无分配器、无 unsafe；块在编译期确定。
//! 契约见 `docs/31-memory-pool.md`。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoolBlock {
    pub index: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemError {
    OutOfMemory,
    InvalidBlock,
    DoubleFree,
}

pub struct BlockPool<const BLOCK_SIZE: usize, const BLOCKS: usize> {
    storage: [[u8; BLOCK_SIZE]; BLOCKS],
    bitmap: u32,
}

impl<const BLOCK_SIZE: usize, const BLOCKS: usize> BlockPool<BLOCK_SIZE, BLOCKS> {
    pub const fn new() -> Self {
        assert!(BLOCKS <= 32, "BlockPool supports at most 32 blocks");
        Self {
            storage: [[0; BLOCK_SIZE]; BLOCKS],
            bitmap: 0,
        }
    }

    pub fn try_alloc(&mut self) -> Result<PoolBlock, MemError> {
        for index in 0..BLOCKS {
            let bit = 1u32 << index;
            if self.bitmap & bit == 0 {
                self.bitmap |= bit;
                return Ok(PoolBlock {
                    index: index as u16,
                });
            }
        }
        Err(MemError::OutOfMemory)
    }

    pub fn free(&mut self, block: PoolBlock) -> Result<(), MemError> {
        let index = usize::from(block.index);
        if index >= BLOCKS {
            return Err(MemError::InvalidBlock);
        }
        let bit = 1u32 << index;
        if self.bitmap & bit == 0 {
            return Err(MemError::DoubleFree);
        }
        self.bitmap &= !bit;
        Ok(())
    }

    pub fn is_allocated(&self, block: PoolBlock) -> bool {
        let index = usize::from(block.index);
        index < BLOCKS && self.bitmap & (1u32 << index) != 0
    }

    pub fn block(&self, block: PoolBlock) -> Option<&[u8; BLOCK_SIZE]> {
        if self.is_allocated(block) {
            Some(&self.storage[usize::from(block.index)])
        } else {
            None
        }
    }

    pub fn block_mut(&mut self, block: PoolBlock) -> Option<&mut [u8; BLOCK_SIZE]> {
        if self.is_allocated(block) {
            Some(&mut self.storage[usize::from(block.index)])
        } else {
            None
        }
    }

    pub fn used_blocks(&self) -> usize {
        self.bitmap.count_ones() as usize
    }

    pub fn free_blocks(&self) -> usize {
        BLOCKS - self.used_blocks()
    }

    pub fn capacity_bytes(&self) -> usize {
        BLOCK_SIZE * BLOCKS
    }

    pub fn used_bytes(&self) -> usize {
        self.used_blocks() * BLOCK_SIZE
    }
}

impl<const BLOCK_SIZE: usize, const BLOCKS: usize> Default for BlockPool<BLOCK_SIZE, BLOCKS> {
    fn default() -> Self {
        Self::new()
    }
}
