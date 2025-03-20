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

pub struct BufferWriter {
    vaddr: VAddr,
    daddr: DAddr,
    byte_offset: usize,
    size: usize,
}

impl BufferWriter {
    pub fn write_bytes(&mut self, data: &[u8]) -> Result<(), Error> {
        let dest = self.reserve(data.len())?;
        dest.copy_from_slice(data);
        Ok(())
    }

    /// Finishes the writes, and returns the start device address of the buffer.
    ///
    /// This requires `self` to ensure you won't write to the buffer anymore
    /// when telling the address to the device. In other words, the buffer will
    /// be moved to the device.
    pub fn finish(self) -> DAddr {
        self.daddr
    }

    pub fn write<T: Copy>(&mut self, value: T) -> Result<(), Error> {
        let dest = self.reserve(1)?;
        dest[0] = value;
        Ok(())
    }

    fn reserve<T: Copy>(&mut self, count: usize) -> Result<&mut [T], Error> {
        if self.byte_offset + count * size_of::<T>() > self.size {
            return Err(Error::OutOfMemory);
        }

        let slice = unsafe {
            let ptr = self.vaddr.add(self.byte_offset).as_mut_ptr();
            core::slice::from_raw_parts_mut(ptr, count)
        };

        self.byte_offset += size_of::<T>() * count;
        Ok(slice)
    }
}

#[derive(Debug)]
pub enum Error {
    OutOfMemory,
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

    pub fn to_device(&mut self) -> Result<BufferWriter, Error> {
        let index = self.allocate().ok_or(Error::OutOfMemory)?;
        let vaddr = self.vaddr(index);
        let daddr = self.daddr(index);

        Ok(BufferWriter {
            vaddr,
            daddr,
            byte_offset: 0,
            size: self.buffer_size,
        })
    }
}
