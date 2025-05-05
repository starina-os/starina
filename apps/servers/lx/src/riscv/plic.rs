use crate::guest_memory::MmioDevice;
use crate::guest_memory::MmioError;

pub fn plic_mmio_size(num_cpus: u32) -> usize {
    0x200000 + (num_cpus as usize * 0x1000)
}

pub struct Plic {}

impl Plic {
    pub fn new() -> Self {
        Self {}
    }
}

impl MmioDevice for Plic {
    fn read(&self, offset: u64, value: &mut [u8]) -> Result<(), MmioError> {
        todo!("plic read: offset={:x}, width={:x}", offset, value.len());
        Ok(())
    }

    fn write(&self, offset: u64, value: &[u8]) -> Result<(), MmioError> {
        todo!("plic write: offset={:x}, width={:x}", offset, value.len());
        Ok(())
    }
}
