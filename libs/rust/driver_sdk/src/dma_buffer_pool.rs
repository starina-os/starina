//! A DMA buffer allocator.
//!
//! This module provides a buffer pool for DMA operations.
use starina::address::PAddr;
use starina::address::VAddr;
use starina::mmio::MmioRegion;
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
/// ```no_run
/// use starina_driver_sdk::DmaBufferPool;
///
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
    folio: MmioRegion,
    free_indices: Vec<BufferId>,
    buffer_size: usize,
    num_buffers: usize,
}

pub struct BufferWriter {
    vaddr: VAddr,
    paddr: PAddr,
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
    pub fn finish(self) -> PAddr {
        self.paddr
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

pub struct BufferReader<'a> {
    slice: &'a [u8],
    byte_offset: usize,
}

impl<'a> BufferReader<'a> {
    pub fn read<T: Copy>(&mut self) -> Result<&T, Error> {
        let slice = self.reserve::<T>(1)?;
        Ok(&slice[0])
    }

    pub fn read_bytes(&mut self, count: usize) -> Result<&[u8], Error> {
        let slice = self.reserve::<u8>(count)?;
        Ok(slice)
    }

    fn reserve<T: Copy>(&mut self, count: usize) -> Result<&[T], Error> {
        let slice = unsafe {
            let bytes_ptr = self.slice.as_ptr().add(self.byte_offset);
            let ptr = bytes_ptr.cast::<T>();
            if !ptr.is_aligned() {
                return Err(Error::AlignmentError);
            }

            core::slice::from_raw_parts(ptr, count)
        };

        self.byte_offset += size_of::<T>() * count;
        Ok(slice)
    }
}

#[derive(Debug)]
pub enum Error {
    OutOfMemory,
    AlignmentError,
}

impl DmaBufferPool {
    pub fn new(buffer_size: usize, num_buffers: usize) -> DmaBufferPool {
        let len = align_up(buffer_size * num_buffers, 4096);
        let folio = MmioRegion::create(len).unwrap();
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
        assert!(index.0 < self.num_buffers, "Invalid buffer index");
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

    /// Returns the device memory address of a buffer.
    pub fn paddr(&self, index: BufferId) -> PAddr {
        debug_assert!(index.0 < self.num_buffers);
        self.folio.paddr().add(index.0 * self.buffer_size)
    }

    pub fn from_device(&mut self, paddr: PAddr) -> Option<BufferReader<'_>> {
        let id = self.paddr_to_id(paddr)?;

        let slice = unsafe {
            let ptr = self.vaddr(id).as_ptr();
            core::slice::from_raw_parts(ptr, self.buffer_size)
        };

        Some(BufferReader {
            slice,
            byte_offset: 0,
        })
    }

    pub fn to_device(&mut self) -> Result<BufferWriter, Error> {
        let index = self.allocate().ok_or(Error::OutOfMemory)?;
        let vaddr = self.vaddr(index);
        let paddr = self.paddr(index);

        Ok(BufferWriter {
            vaddr,
            paddr,
            byte_offset: 0,
            size: self.buffer_size,
        })
    }
}
