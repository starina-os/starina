use core::slice;

use starina::address::GPAddr;
use starina::address::VAddr;
use starina::error::ErrorCode;
use starina::folio::Folio;
use starina::hvspace::HvSpace;
use starina::prelude::*;
use starina::vmspace::PageProtect;
use starina::vmspace::VmSpace;

use crate::riscv::plic::Plic;
use crate::virtio;
use crate::virtio::device::VIRTIO_MMIO_SIZE;
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

#[derive(Debug)]
pub enum MmioError {
    NotMapped,
    NotMmio,
}

pub trait MmioDevice {
    fn read(&self, offset: u64, value: &mut [u8]) -> Result<(), MmioError>;
    fn write(&self, offset: u64, value: &[u8]) -> Result<(), MmioError>;
}

enum Backend {
    Ram(Ram),
    VirtioMmio(VirtioMmio),
    Plic(Plic),
}

struct Mapping {
    start: GPAddr,
    end: GPAddr,
    backend: Backend,
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

        self.mappings.push(Mapping {
            start: ram.gpaddr,
            end: ram.gpaddr.checked_add(ram.size).unwrap(),
            backend: Backend::Ram(ram),
        });
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
        self.mappings.push(Mapping {
            start: gpaddr,
            end: gpaddr.checked_add(VIRTIO_MMIO_SIZE).unwrap(),
            backend: Backend::VirtioMmio(device),
        });

        Ok(())
    }

    pub fn add_plic(&mut self, gpaddr: GPAddr, mmio_size: usize, plic: Plic) -> Result<(), Error> {
        self.mappings.push(Mapping {
            start: gpaddr,
            end: gpaddr.checked_add(mmio_size).unwrap(),
            backend: Backend::Plic(plic),
        });
        Ok(())
    }

    fn find_mmio_device(&self, gpaddr: GPAddr) -> Result<(&dyn MmioDevice, u64), MmioError> {
        for mapping in &self.mappings {
            if mapping.start <= gpaddr && gpaddr < mapping.end {
                let device = match &mapping.backend {
                    Backend::Ram(_) => {
                        return Err(MmioError::NotMmio);
                    }
                    Backend::VirtioMmio(device) => device as &dyn MmioDevice,
                    Backend::Plic(plic) => plic as &dyn MmioDevice,
                };

                let offset = gpaddr.as_usize() - mapping.start.as_usize();
                return Ok((device, offset as u64));
            }
        }

        Err(MmioError::NotMapped)
    }

    pub fn mmio_read(&self, gpaddr: GPAddr, value: &mut [u8]) -> Result<(), MmioError> {
        let (device, offset) = self.find_mmio_device(gpaddr)?;
        device.read(offset, value)?;
        Ok(())
    }

    pub fn mmio_write(&self, gpaddr: GPAddr, value: &[u8]) -> Result<(), MmioError> {
        let (device, offset) = self.find_mmio_device(gpaddr)?;
        device.write(offset, value)?;
        Ok(())
    }
}
