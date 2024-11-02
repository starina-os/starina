//! A contiguous page-aliged memory block.
use starina_types::address::PAddr;
use starina_types::address::VAddr;
use starina_types::error::FtlError;
use starina_types::vmspace::PageProtect;
use starina_utils::alignment::align_down;
use starina_utils::alignment::align_up;

use crate::handle::OwnedHandle;
use crate::start::app_vmspace_handle;
use crate::syscall;

/// The ownership of a contiguous page-aliged memory region.
///
/// To summarize:
///
/// - The memory block address is page-aligned (typically 4KB).
/// - The memory block size is also page-aligned.
/// - The memory block is *physically* contiguous.
///
/// # When to use
///
/// Use folio when you need a *physically contiguous* memory region. The common
/// case is when you need to allocate a DMA buffer in a device driver (strictly
/// speaking, when IOMMU is not available).
///
/// # Prefer [`Box<T>`](crate::prelude::Box) over folio
///
/// Unless you need low-level control over memory allocation, use containers
/// like [`Vec<T>`](crate::prelude::Vec) or [`Box<T>`](crate::prelude::Box)
/// memory regions directly, such as DMA buffers, MMIO regions, and shared
/// instead of folio. Folio is intended for OS services that need to manage
/// memory between processes.
///
/// # You may want [`MappedFolio`] instead
///
/// If you want to access the memory region, use [`MappedFolio`] instead.
///
/// # Why "folio"?
///
/// Because it's *a sheet of paper (pages)*.
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

    /// Allocates a folio at a specific physical address (`paddr`), and maps it to the
    /// current process's address space.
    pub fn create_pinned(paddr: PAddr, len: usize) -> Result<MappedFolio, FtlError> {
        let offset = paddr.as_usize() % 4096; // FIXME:
        let map_paddr = PAddr::new(align_down(paddr.as_usize(), 4096));
        let map_len = align_up(len, 4096);

        let handle = syscall::folio_create_fixed(map_paddr, map_len)?;
        let vaddr = syscall::vmspace_map(
            app_vmspace_handle(),
            map_len,
            handle,
            PageProtect::READABLE | PageProtect::WRITABLE,
        )?;

        Ok(MappedFolio {
            _folio: Folio {
                handle: OwnedHandle::from_raw(handle),
            },
            paddr,
            vaddr: vaddr.add(offset),
        })
    }

    /// Returns the start address of the folio in the current process's address space.
    pub fn vaddr(&self) -> VAddr {
        self.vaddr
    }

    /// Returns the start address of the folio in physical memory space.
    pub fn paddr(&self) -> PAddr {
        self.paddr
    }
}
