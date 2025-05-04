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

pub const VIRTIO_MMIO_SIZE: usize = 4096;

#[derive(Debug)]
pub enum Error {
    AllocFolio(ErrorCode),
    VmSpaceMap(ErrorCode),
}

/// Virtio device over memory-mapped I/O.
pub struct VirtioMmio {
    device: Box<dyn VirtioDevice>,
}

impl VirtioMmio {
    pub fn new(device: impl VirtioDevice + 'static) -> Result<Self, Error> {
        Ok(Self {
            device: Box::new(device),
        })
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
