use starina::error::ErrorCode;
use starina::prelude::*;

use crate::guest_memory::MmioDevice;
use crate::guest_memory::MmioError;

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
}

impl MmioDevice for VirtioMmio {
    fn read(&self, offset: u64, value: &mut [u8]) -> Result<(), MmioError> {
        todo!(
            "virtio mmio read: offset={:x}, width={:x}",
            offset,
            value.len()
        );
        Ok(())
    }

    fn write(&self, offset: u64, value: &[u8]) -> Result<(), MmioError> {
        todo!(
            "virtio mmio write: offset={:x}, width={:x}",
            offset,
            value.len()
        );
        Ok(())
    }
}
