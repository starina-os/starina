use starina::address::VAddr;
use starina::error::ErrorCode;
use starina::folio::Folio;
use starina::prelude::*;
use starina::vmspace::PageProtect;
use starina::vmspace::VmSpace;

/// The host-side (device-side) of a virtio device.
///
/// Guest OS interacts with this device through their virtio
/// device drivers.
pub trait VirtioDevice {}

#[derive(Debug)]
pub enum Error {
    AllocFolio(ErrorCode),
    VmSpaceMap(ErrorCode),
}

/// Virtio device over memory-mapped I/O.
pub struct VirtioMmio {
    device: Box<dyn VirtioDevice>,
    mmio_folio: Folio,
    mmio_vaddr: VAddr,
    mmio_size: usize,
}

impl VirtioMmio {
    pub fn new(device: impl VirtioDevice + 'static) -> Result<Self, Error> {
        let mmio_size = 4096;
        let mmio_folio = Folio::alloc(mmio_size).map_err(Error::AllocFolio)?;
        let mmio_vaddr = VmSpace::map_anywhere_current(
            &mmio_folio,
            mmio_size,
            PageProtect::READABLE | PageProtect::WRITEABLE,
        )
        .map_err(Error::VmSpaceMap)?;

        Ok(Self {
            device: Box::new(device),
            mmio_folio,
            mmio_vaddr,
            mmio_size,
        })
    }

    pub fn mmio_folio(&self) -> &Folio {
        &self.mmio_folio
    }

    pub fn mmio_size(&self) -> usize {
        self.mmio_size
    }

    pub fn mmio_read(&self, offset: usize, width: usize) -> u64 {
        match offset {
            _ => {
                panic!("unsupported offset: {}", offset);
            }
        }
    }

    pub fn mmio_write(&self, offset: usize, width: usize, value: u64) {
        match offset {
            _ => {
                panic!("unsupported offset: {}", offset);
            }
        }
    }
}
