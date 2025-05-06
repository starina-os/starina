use core::slice;

use starina::address::GPAddr;
use starina::address::VAddr;
use starina::error::ErrorCode;
use starina::folio::Folio;
use starina::hvspace::HvSpace;
use starina::prelude::*;
use starina::vmspace::PageProtect;
use starina::vmspace::VmSpace;

#[derive(Debug)]
pub enum Error {
    AllocFolio(ErrorCode),
    CreateHvSpace(ErrorCode),
    VmSpaceMap(ErrorCode),
    MapRam(ErrorCode),
    OutOfRam,
}

fn align_up(size: usize, align: usize) -> usize {
    (size + align - 1) & !(align - 1)
}

pub struct GuestMemory {
    hvspace: HvSpace,
    start: GPAddr,
    end: GPAddr,
    size: usize,
    folio: Folio,
    vaddr: VAddr,
    free_offset: usize,
}

impl GuestMemory {
    pub fn new(start: GPAddr, size: usize) -> Result<Self, Error> {
        let end = start.checked_add(size).unwrap();

        // Allocate a virtually-contiguous memory region (folio).
        let folio = Folio::alloc(size).map_err(Error::AllocFolio)?;

        // Map the folio into the current (VMM's) address space.
        let vaddr = VmSpace::map_anywhere_current(
            &folio,
            size,
            PageProtect::READABLE | PageProtect::WRITEABLE,
        )
        .map_err(Error::VmSpaceMap)?;

        // Create a guest address space and map the folio into it.
        let hvspace = HvSpace::new().map_err(Error::CreateHvSpace)?;
        hvspace
            .map(
                start,
                &folio,
                size,
                PageProtect::READABLE | PageProtect::WRITEABLE | PageProtect::EXECUTABLE,
            )
            .map_err(Error::MapRam)?;

        Ok(Self {
            hvspace,
            start,
            end,
            size,
            folio,
            vaddr,
            free_offset: 0,
        })
    }

    pub fn hvspace(&self) -> &HvSpace {
        &self.hvspace
    }

    pub fn allocate(&mut self, size: usize, align: usize) -> Result<(&mut [u8], GPAddr), Error> {
        let free_start = align_up(self.free_offset, align);
        if free_start + size > self.size {
            return Err(Error::OutOfRam);
        }

        self.free_offset = free_start + size;

        let gpaddr = self.start.checked_add(free_start).unwrap();
        let slice = &mut self.bytes_mut()[free_start..free_start + size];

        trace!("RAM: allocated {} bytes at {}", size, gpaddr);
        Ok((slice, gpaddr))
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: The folio is mapped to the current vmspace, and folio
        // is kept alive as long as `self` is alive.
        unsafe { slice::from_raw_parts_mut(self.vaddr.as_mut_ptr(), self.size) }
    }
}
