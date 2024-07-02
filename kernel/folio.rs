use core::borrow::Borrow;

use ftl_types::address::PAddr;
use ftl_types::address::VAddr;
use ftl_types::error::FtlError;

use crate::arch;
use crate::cpuvar::current_thread;
use crate::memory::AllocPagesError;
use crate::memory::AllocatedPages;

enum Pages {
    Anonymous(AllocatedPages),
    Mmio { paddr: PAddr },
}

pub struct Folio {
    #[allow(dead_code)]
    pages: Pages,
    len: usize,
}

impl Folio {
    pub fn alloc(len: usize) -> Result<Folio, AllocPagesError> {
        let pages = AllocatedPages::alloc(len)?;
        Ok(Self::from_allocated_pages(pages))
    }

    pub fn alloc_mmio(paddr: PAddr, len: usize) -> Result<Folio, AllocPagesError> {
        Ok(Folio {
            pages: Pages::Mmio { paddr },
            len,
        })
    }

    pub fn from_allocated_pages(pages: AllocatedPages) -> Folio {
        let len = pages.len();
        Folio {
            pages: Pages::Anonymous(pages),
            len,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn vaddr(&self) -> Result<VAddr, FtlError> {
        if current_thread().borrow().process().is_kernel_process() {
            let vaddr = match &self.pages {
                Pages::Anonymous(pages) => pages.as_vaddr(),
                Pages::Mmio { paddr } => arch::paddr2vaddr(*paddr).ok_or(FtlError::InvalidArg)?,
            };

            Ok(vaddr)
        } else {
            // TODO: Map this folio into the current process' address space.
            Err(FtlError::NotSupported)
        }
    }

    pub fn paddr(&self) -> Result<PAddr, FtlError> {
        match &self.pages {
            Pages::Mmio { paddr, .. } => Ok(*paddr),
            _ => Err(FtlError::NotSupported),
        }
    }
}
