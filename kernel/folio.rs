//! Folio, a physically-contiguous memory region.
use core::alloc::GlobalAlloc;
use core::alloc::Layout;

use ftl_types::address::PAddr;
use ftl_types::address::VAddr;
use ftl_types::error::FtlError;
use ftl_utils::alignment::is_aligned;

use crate::arch::vaddr2paddr;
use crate::arch::PAGE_SIZE;
use crate::memory::GLOBAL_ALLOCATOR;

pub struct Folio {
    paddr: PAddr,
    len: usize,
}

impl Folio {
    pub fn alloc(len: usize) -> Result<Folio, FtlError> {
        if len == 0 || !is_aligned(len, PAGE_SIZE) {
            return Err(FtlError::InvalidArg);
        }

        let layout = match Layout::from_size_align(len, PAGE_SIZE) {
            Ok(layout) => layout,
            Err(_) => {
                return Err(FtlError::InvalidArg);
            }
        };

        // SAFETY: `len` is not zero as checked above.
        let ptr = unsafe { GLOBAL_ALLOCATOR.alloc(layout) };

        // Fill the allocated memory with zeros.
        unsafe {
            core::ptr::write_bytes(ptr, 0, len);
        }

        let folio = Self {
            paddr: vaddr2paddr(VAddr::new(ptr as usize)).unwrap(),
            len,
        };

        folio.inc_ref();
        Ok(folio)
    }

    pub fn alloc_shared(paddr: PAddr, len: usize) -> Result<Folio, FtlError> {
        if len == 0 || !is_aligned(len, PAGE_SIZE) {
            return Err(FtlError::InvalidArg);
        }

        if !is_aligned(paddr.as_usize(), PAGE_SIZE) {
            return Err(FtlError::InvalidArg);
        }

        // TODO: Inherit the reference counter if the paddr is already owned by a folio.
        // TODO: Make sure the paddr range is not exclusively owned by any other folio.

        let folio = Self { paddr, len };

        folio.inc_ref();
        Ok(folio)
    }

    pub fn alloc_fixed(paddr: PAddr, len: usize) -> Result<Folio, FtlError> {
        if len == 0 || !is_aligned(len, PAGE_SIZE) {
            return Err(FtlError::InvalidArg);
        }

        if !is_aligned(paddr.as_usize(), PAGE_SIZE) {
            return Err(FtlError::InvalidArg);
        }

        // TODO: Make sure the paddr range is not owned by any other folio.

        let folio = Self { paddr, len };

        folio.inc_ref();
        Ok(folio)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn paddr(&self) -> PAddr {
        self.paddr
    }

    pub fn inc_ref(&self) {
        // TODO:
    }

    pub fn dec_ref(&self) {
        // TODO:
    }
}

impl Drop for Folio {
    fn drop(&mut self) {
        self.dec_ref();
    }
}
