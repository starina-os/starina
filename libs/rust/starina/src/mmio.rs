//! A contiguous page-aliged memory block.
use starina_types::address::PAddr;
use starina_types::address::VAddr;
use starina_types::error::ErrorCode;
use starina_types::vmspace::PageProtect;
use starina_utils::alignment::is_aligned;

use crate::folio::Folio;
use crate::handle::Handleable;
use crate::syscall;
use crate::vmspace::SELF_VMSPACE;

pub struct MmioRegion {
    _folio: Folio,
    paddr: PAddr,
    vaddr: VAddr,
    len: usize,
}

impl MmioRegion {
    /// Allocates a folio at an arbitrary physical address, and maps it to the
    /// current process's address space.
    pub fn create(len: usize) -> Result<MmioRegion, ErrorCode> {
        debug_assert!(is_aligned(len, 0x1000));

        let folio = Folio::alloc(len)?;
        let vaddr = syscall::vmspace_map(
            SELF_VMSPACE,
            VAddr::new(0), /* anywhere */
            len,
            folio.handle_id(),
            0,
            PageProtect::READABLE | PageProtect::WRITEABLE,
        )?;

        let paddr = folio.paddr()?;

        Ok(MmioRegion {
            _folio: folio,
            paddr,
            vaddr,
            len,
        })
    }

    /// Allocates a folio at a specific physical address (`paddr`), and maps it to the
    /// current process's address space.
    pub fn pin(paddr: PAddr, len: usize) -> Result<MmioRegion, ErrorCode> {
        debug_assert!(is_aligned(paddr.as_usize(), 0x1000));
        debug_assert!(is_aligned(len, 0x1000));

        let folio = Folio::pin(paddr, len)?;
        let vaddr = syscall::vmspace_map(
            SELF_VMSPACE,
            VAddr::new(0), /* anywhere */
            len,
            folio.handle_id(),
            0,
            PageProtect::READABLE | PageProtect::WRITEABLE,
        )?;

        Ok(MmioRegion {
            _folio: folio,
            paddr,
            vaddr,
            len,
        })
    }

    //// # Safety
    ///
    /// <https://doc.rust-lang.org/std/ptr/index.html#pointer-to-reference-conversion>
    pub unsafe fn as_ref<T: Copy>(&self, byte_offset: usize) -> &T {
        assert!(byte_offset + size_of::<T>() <= self.len);
        assert!(is_aligned(byte_offset, align_of::<T>()));

        let ptr = unsafe { self.vaddr.add(byte_offset).as_ptr::<T>() };
        unsafe { &*ptr }
    }

    //// # Safety
    ///
    /// <https://doc.rust-lang.org/std/ptr/index.html#pointer-to-reference-conversion>
    pub unsafe fn as_mut<T: Copy>(&mut self, byte_offset: usize) -> &mut T {
        assert!(byte_offset + size_of::<T>() <= self.len);
        assert!(is_aligned(byte_offset, align_of::<T>()));

        let ptr = unsafe { self.vaddr.add(byte_offset).as_mut_ptr::<T>() };
        unsafe { &mut *ptr }
    }

    pub fn vaddr(&self) -> VAddr {
        self.vaddr
    }

    /// Returns the start address of the folio in device memory space.
    pub fn paddr(&self) -> PAddr {
        self.paddr
    }
}
