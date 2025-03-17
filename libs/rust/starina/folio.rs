//! A contiguous page-aliged memory block.
use starina_types::address::DAddr;
use starina_types::address::PAddr;
use starina_types::address::VAddr;
use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
use starina_types::vmspace::PageProtect;
use starina_utils::alignment::align_down;
use starina_utils::alignment::align_up;

use crate::handle::OwnedHandle;
use crate::syscall;

// FIXME: What if PAGE_SIZE is not 4KB?
const PAGE_SIZE: usize = 4096;

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
    pub fn create(len: usize) -> Result<Folio, ErrorCode> {
        let handle = syscall::folio_create(len)?;
        Ok(Folio {
            handle: OwnedHandle::from_raw(handle),
        })
    }

    pub fn handle(&self) -> &OwnedHandle {
        &self.handle
    }

    pub fn paddr(&self) -> Result<PAddr, ErrorCode> {
        let paddr = syscall::folio_paddr(self.handle.id())?;
        Ok(paddr)
    }
}

const SELF_VMSPACE: HandleId = HandleId::from_raw(0);

pub struct MmioFolio {
    _folio: Folio,
    daddr: DAddr,
    vaddr: VAddr,
}

impl MmioFolio {
    /// Allocates a folio at an arbitrary physical address, and maps it to the
    /// current process's address space.
    pub fn create(bus: IoBus, len: usize) -> Result<MmioFolio, ErrorCode> {
        let handle = syscall::folio_create(len)?;
        let daddr = bus.map(SELF_VMSPACE, None, len)?;
        Ok(MmioFolio {
            _folio: Folio {
                handle: OwnedHandle::from_raw(handle),
            },
            daddr,
            vaddr,
        })
    }

    /// Allocates a folio at a specific physical address (`paddr`), and maps it to the
    /// current process's address space.
    pub fn create_pinned(bus: IoBus, daddr: DAddr, len: usize) -> Result<MmioFolio, ErrorCode> {
        let offset = daddr.as_usize() % PAGE_SIZE;
        let map_daddr = DAddr::new(align_down(daddr.as_usize(), PAGE_SIZE));
        let map_len = align_up(len, PAGE_SIZE);

        let daddr = bus.map(SELF_VMSPACE, Some(map_daddr), map_len)?;
        Ok(MmioFolio {
            _folio: Folio {
                handle: OwnedHandle::from_raw(handle),
            },
            daddr,
            vaddr: vaddr.add(offset),
        })
    }

    /// Returns the start address of the folio in the current process's address space.
    pub fn vaddr(&self) -> VAddr {
        self.vaddr
    }

    /// Returns the start address of the folio in device memory space.
    pub fn daddr(&self) -> DAddr {
        self.daddr
    }
}
