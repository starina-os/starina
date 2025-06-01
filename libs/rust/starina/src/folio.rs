//! A contiguous page-aliged memory block.
use starina_types::address::PAddr;
use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
use starina_utils::alignment::is_aligned;

use crate::handle::Handleable;
use crate::handle::OwnedHandle;
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
    pub fn alloc(size: usize) -> Result<Folio, ErrorCode> {
        assert!(is_aligned(size, 0x1000));

        let id = syscall::folio_alloc(size)?;
        let handle = OwnedHandle::from_raw(id);
        Ok(Folio { handle })
    }

    pub fn pin(paddr: PAddr, size: usize) -> Result<Folio, ErrorCode> {
        assert!(is_aligned(size, 0x1000));

        let id = syscall::folio_pin(paddr, size)?;
        let handle = OwnedHandle::from_raw(id);
        Ok(Folio { handle })
    }

    pub const fn from_handle(handle: OwnedHandle) -> Self {
        Self { handle }
    }

    pub fn paddr(&self) -> Result<PAddr, ErrorCode> {
        syscall::folio_paddr(self.handle.id())
    }
}

impl Handleable for Folio {
    fn handle_id(&self) -> HandleId {
        self.handle.id()
    }
}

/// Returns the page size.
///
/// # Why not a constant?
///
/// To make it easy to support non-4KB pages, reading it from the kernel
/// (ala `sysconf(_SC_PAGESIZE)`) in the future.
pub fn page_size() -> usize {
    4096
}
