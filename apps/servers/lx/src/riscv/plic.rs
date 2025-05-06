use starina::prelude::*;

use crate::guest_memory::GuestMemory;
use crate::mmio;
use crate::mmio::Device;

pub const fn plic_mmio_size(num_cpus: u32) -> usize {
    0x200000 + (num_cpus as usize * 0x1000)
}

pub struct Plic {}

impl Plic {
    pub fn new() -> Self {
        Self {}
    }
}

impl Device for Plic {
    fn mmio_read(
        &self,
        _memory: &mut GuestMemory,
        offset: u64,
        value: &mut [u8],
    ) -> Result<(), mmio::Error> {
        trace!("plic read: offset={:x}, width={:x}", offset, value.len());
        Ok(())
    }

    fn mmio_write(
        &self,
        _memory: &mut GuestMemory,
        offset: u64,
        value: &[u8],
    ) -> Result<(), mmio::Error> {
        trace!("plic write: offset={:x}, width={:x}", offset, value.len());
        Ok(())
    }
}
