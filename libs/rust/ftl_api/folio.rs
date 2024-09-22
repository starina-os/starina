use ftl_types::address::PAddr;
use ftl_types::address::VAddr;
use ftl_types::error::FtlError;
use ftl_types::vmspace::PageProtect;

use crate::handle::OwnedHandle;
use crate::start::app_vmspace_handle;
use crate::syscall;

pub struct Folio {
    handle: OwnedHandle,
}

impl Folio {
    pub fn create(len: usize) -> Result<Folio, FtlError> {
        let handle = syscall::folio_create(len)?;
        Ok(Folio {
            handle: OwnedHandle::from_raw(handle),
        })
    }

    pub fn handle(&self) -> &OwnedHandle {
        &self.handle
    }

    pub fn paddr(&self) -> Result<PAddr, FtlError> {
        let paddr = syscall::folio_paddr(self.handle.id())?;
        let paddr = PAddr::new(paddr);
        Ok(paddr)
    }
}

/// A folio mapped to the current process's address space.
pub struct MappedFolio {
    _folio: Folio,
    paddr: PAddr,
    vaddr: VAddr,
}

impl MappedFolio {
    /// Allocates a folio at an arbitrary physical address, and maps it to the
    /// current process's address space.
    pub fn create(len: usize) -> Result<MappedFolio, FtlError> {
        let handle = syscall::folio_create(len)?;
        let vaddr = syscall::vmspace_map(
            app_vmspace_handle(),
            len,
            handle,
            PageProtect::READABLE | PageProtect::WRITABLE,
        )?;
        let paddr = syscall::folio_paddr(handle)?;
        Ok(MappedFolio {
            _folio: Folio {
                handle: OwnedHandle::from_raw(handle),
            },
            paddr: PAddr::new(paddr),
            vaddr,
        })
    }

    pub fn create_pinned(paddr: PAddr, len: usize) -> Result<MmioFolio, FtlError> {
        let handle = syscall::folio_create_fixed(paddr, len)?;
        let vaddr = syscall::vmspace_map(
            app_vmspace_handle(),
            len,
            handle,
            PageProtect::READABLE | PageProtect::WRITABLE,
        )?;

        Ok(MappedFolio {
            _folio: Folio {
                handle: OwnedHandle::from_raw(handle),
            },
            paddr,
            vaddr,
        })
    }

    pub fn vaddr(&self) -> VAddr {
        self.vaddr
    }

    pub fn paddr(&self) -> PAddr {
        self.paddr
    }
}
