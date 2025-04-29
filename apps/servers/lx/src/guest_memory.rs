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
}

pub struct Ram {
    folio: Folio,
    vaddr: VAddr,
    size: usize,
}

impl Ram {
    pub fn new(size: usize) -> Result<Self, Error> {
        let folio = Folio::alloc(size).map_err(Error::AllocFolio)?;
        let vaddr = VmSpace::map_anywhere_current(
            &folio,
            size,
            PageProtect::READABLE | PageProtect::WRITEABLE,
        )
        .map_err(Error::VmSpaceMap)?;

        Ok(Self { folio, vaddr, size })
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: The folio is mapped to the current vmspace, and folio
        // is kept alive as long as `self` is alive.
        unsafe { slice::from_raw_parts_mut(self.vaddr.as_mut_ptr(), self.size) }
    }
}

enum Mapping {
    Ram(Ram),
}

pub struct GuestMemory {
    mappings: Vec<Mapping>,
    hvspace: HvSpace,
}

impl GuestMemory {
    pub fn new() -> Result<Self, Error> {
        let hvspace = HvSpace::new().map_err(Error::CreateHvSpace)?;
        Ok(Self {
            hvspace,
            mappings: Vec::new(),
        })
    }

    pub fn hvspace(&self) -> &HvSpace {
        &self.hvspace
    }

    pub fn map_ram(&mut self, gpaddr: GPAddr, ram: Ram) -> Result<(), Error> {
        self.hvspace
            .map(
                gpaddr,
                &ram.folio,
                ram.size,
                PageProtect::READABLE | PageProtect::WRITEABLE,
            )
            .map_err(Error::MapRam)?;

        self.mappings.push(Mapping::Ram(ram));
        Ok(())
    }
}
