//! A DMA buffer allocator.
//!
//! This module provides a buffer pool for DMA operations.
use starina_api::folio::MappedFolio;
use starina_api::prelude::vec::Vec;
use starina_api::types::address::PAddr;
use starina_api::types::address::VAddr;
use starina_utils::alignment::align_up;

/// A buffer identifier.
#[derive(Copy, Clone)]
pub struct BufferId(usize);

/// A DMA buffer pool.
///
/// This struct manages a pool of buffers. Unlike a `Vec`-based buffers, this
/// struct provides a way to know the physical memory address of a buffer so
/// that it can be passed to a device for DMA operations.
///
/// # Future Work
///
/// - Distinguish the physical memory address and device memory address. Some
///   computers might have different address spaces for devices, and some might
///   have IOMMU to translate the addresses.
///
/// # Example
///
/// ```rust
/// const BUFFER_SIZE: usize = 4096;
/// const NUM_BUFFERS: usize = 16;
///
/// let mut pool = DmaBufferPool::new(BUFFER_SIZE, NUM_BUFFERS);
/// let buffer_id = pool.allocate().unwrap();
///
/// let paddr = pool.paddr(buffer_id);
/// let vaddr = pool.vaddr(buffer_id);
///
/// // Do DMA operations here!
///
/// pool.free(buffer_id);
/// ```
pub struct DmaBufferPool {
    folio: MappedFolio,
    free_indices: Vec<BufferId>,
    buffer_size: usize,
    num_buffers: usize,
}

impl DmaBufferPool {
    pub fn new(buffer_size: usize, num_buffers: usize) -> DmaBufferPool {
        let folio = MappedFolio::create(align_up(buffer_size * num_buffers, 4096)).unwrap();
        let mut free_indices = Vec::new();
        for i in 0..num_buffers {
            free_indices.push(BufferId(i));
        }

        DmaBufferPool {
            folio,
            free_indices,
            buffer_size,
            num_buffers,
        }
    }

    /// Allocates a buffer.
    pub fn allocate(&mut self) -> Option<BufferId> {
        self.free_indices.pop()
    }

    /// Frees a buffer.
    pub fn free(&mut self, index: BufferId) {
        self.free_indices.push(index);
    }

    /// Converts a physical memory address to a buffer index.
    pub fn paddr_to_id(&self, paddr: PAddr) -> Option<BufferId> {
        debug_assert!(
            paddr.as_usize() % self.buffer_size == 0,
            "paddr is not aligned"
        );

        // TODO: paddr may not be in the same folio
        let offset = paddr.as_usize() - self.folio.paddr().as_usize();
        let index = offset / self.buffer_size;
        if index < self.num_buffers {
            Some(BufferId(index))
        } else {
            None
        }
    }

    /// Returns the virtual memory address of a buffer.
    pub fn vaddr(&self, index: BufferId) -> VAddr {
        debug_assert!(index.0 < self.num_buffers);
        self.folio.vaddr().add(index.0 * self.buffer_size)
    }

    /// Returns the physical memory address of a buffer.
    pub fn paddr(&self, index: BufferId) -> PAddr {
        debug_assert!(index.0 < self.num_buffers);
        self.folio.paddr().add(index.0 * self.buffer_size)
    }
}
