use core::slice;

use starina::address::GPAddr;
use starina::address::VAddr;
use starina::error::ErrorCode;
use starina::folio::Folio;
use starina::hvspace::HvSpace;
use starina::prelude::*;
use starina::vmspace::PageProtect;
use starina::vmspace::VmSpace;

use crate::virtio;
use crate::virtio::device::VirtioDevice;
use crate::virtio::device::VirtioMmio;

#[derive(Debug)]
pub enum Error {
    AllocFolio(ErrorCode),
    CreateHvSpace(ErrorCode),
    VmSpaceMap(ErrorCode),
    MapRam(ErrorCode),
    CreateVirtioMmio(virtio::device::Error),
    MapVirtioMmio(ErrorCode),
    OutOfRam,
}

fn align_up(size: usize, align: usize) -> usize {
    (size + align - 1) & !(align - 1)
}

pub struct Ram {
    folio: Folio,
    gpaddr: GPAddr,
    vaddr: VAddr,
    size: usize,
    free_offset: usize,
}

impl Ram {
    pub fn new(gpaddr: GPAddr, size: usize) -> Result<Self, Error> {
        let folio = Folio::alloc(size).map_err(Error::AllocFolio)?;
        let vaddr = VmSpace::map_anywhere_current(
            &folio,
            size,
            PageProtect::READABLE | PageProtect::WRITEABLE,
        )
        .map_err(Error::VmSpaceMap)?;

        Ok(Self {
            folio,
            gpaddr,
            vaddr,
            size,
            free_offset: 0,
        })
    }

    pub fn allocate(&mut self, size: usize, align: usize) -> Result<(&mut [u8], GPAddr), Error> {
        let free_start = align_up(self.free_offset, align);
        if free_start + size > self.size {
            return Err(Error::OutOfRam);
        }

        self.free_offset = free_start + size;

        let gpaddr = self.gpaddr.checked_add(free_start).unwrap();
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

enum Mapping {
    Ram(Ram),
    VirtioMmio(VirtioMmio),
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

    pub fn add_ram(&mut self, ram: Ram) -> Result<(), Error> {
        info!("guest_memory: mapping RAM at {}", ram.gpaddr);
        self.hvspace
            .map(
                ram.gpaddr,
                &ram.folio,
                ram.size,
                PageProtect::READABLE | PageProtect::WRITEABLE | PageProtect::EXECUTABLE,
            )
            .map_err(Error::MapRam)?;

        self.mappings.push(Mapping::Ram(ram));
        Ok(())
    }

    pub fn add_virtio_mmio(
        &mut self,
        gpaddr: GPAddr,
        device: impl VirtioDevice + 'static,
    ) -> Result<(), Error> {
        info!("guest_memory: mapping virtio-mmio at {}", gpaddr);
        // Do not map the folio into the hvspace; we'll intentionally let CPUs
        // cause page faults on MMIO addresses to handle them programmatically.
        let device = VirtioMmio::new(device).map_err(Error::CreateVirtioMmio)?;
        self.mappings.push(Mapping::VirtioMmio(device));
        Ok(())
    }
}
