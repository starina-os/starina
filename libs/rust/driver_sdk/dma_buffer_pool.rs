//! A DMA buffer allocator.
//!
//! This module provides a buffer pool for DMA operations.
use starina::address::DAddr;
use starina::address::VAddr;
use starina::folio::MmioFolio;
use starina::iobus::IoBus;
use starina::prelude::vec::Vec;
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
/// let daddr = pool.daddr(buffer_id);
/// let vaddr = pool.vaddr(buffer_id);
///
/// // Do DMA operations here!
///
/// pool.free(buffer_id);
/// ```
pub struct DmaBufferPool {
    folio: MmioFolio,
    free_indices: Vec<BufferId>,
    buffer_size: usize,
    num_buffers: usize,
}

impl DmaBufferPool {
    pub fn new(iobus: &IoBus, buffer_size: usize, num_buffers: usize) -> DmaBufferPool {
        let len = align_up(buffer_size * num_buffers, 4096);
        let folio = MmioFolio::create(iobus, len).unwrap();
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
    pub fn daddr_to_id(&self, daddr: DAddr) -> Option<BufferId> {
        debug_assert!(
            daddr.as_usize() % self.buffer_size == 0,
            "daddr is not aligned"
        );

        // TODO: daddr may not be in the same folio
        let offset = daddr.as_usize() - self.folio.daddr().as_usize();
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

    /// Returns the device memory address of a buffer.
    pub fn daddr(&self, index: BufferId) -> DAddr {
        debug_assert!(index.0 < self.num_buffers);
        self.folio.daddr().add(index.0 * self.buffer_size)
    }
}
