use core::alloc::GlobalAlloc;
use core::alloc::Layout;

use ftl_types::address::PAddr;
use ftl_types::address::VAddr;
use ftl_types::error::FtlError;
use ftl_utils::alignment::is_aligned;

use crate::arch::paddr2vaddr;
use crate::arch::vaddr2paddr;
use crate::arch::PAGE_SIZE;
use crate::memory::GLOBAL_ALLOCATOR;

#[derive(Debug)]
enum PageType {
    Allocated { layout: Layout },
    Fixed,
}

pub struct Folio {
    page_type: PageType,
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

        Ok(Self {
            page_type: PageType::Allocated { layout },
            paddr: vaddr2paddr(VAddr::new(ptr as usize)).unwrap(),
            len,
        })
    }

    pub fn alloc_fixed(paddr: PAddr, len: usize) -> Result<Folio, FtlError> {
        if len == 0 || !is_aligned(len, PAGE_SIZE) {
            return Err(FtlError::InvalidArg);
        }

        if !is_aligned(paddr.as_usize(), PAGE_SIZE) {
            return Err(FtlError::InvalidArg);
        }

        Ok(Self {
            page_type: PageType::Fixed,
            paddr,
            len,
        })
    }

    pub fn len(&self) -> usize {
        self.len
    }
    pub fn paddr(&self) -> PAddr {
        self.paddr
    }
}

impl Drop for Folio {
    fn drop(&mut self) {
        match self.page_type {
            PageType::Allocated { layout } => {
                // SAFETY: `layout` is the same as the one used in `alloc` method.
                let vaddr = match paddr2vaddr(self.paddr) {
                    Ok(vaddr) => vaddr,
                    Err(err) => {
                        warn!("failed to paddr2vaddr while dropping a folio: {:?}", err);
                        return;
                    }
                };

                unsafe { GLOBAL_ALLOCATOR.dealloc(vaddr.as_mut_ptr(), layout) };
            }
            PageType::Fixed => {}
        }
    }
}
