use starina::address::GPAddr;
use starina::prelude::*;

use crate::guest_memory::GuestMemory;

#[derive(Debug)]
pub enum Error {
    NotMapped,
}

pub trait Device {
    fn mmio_read(
        &self,
        memory: &mut GuestMemory,
        offset: u64,
        value: &mut [u8],
    ) -> Result<(), Error>;
    fn mmio_write(&self, memory: &mut GuestMemory, offset: u64, value: &[u8]) -> Result<(), Error>;
}

struct Region {
    start: GPAddr,
    end: GPAddr,
    device: Box<dyn Device>,
}

impl Region {
    pub fn contains(&self, gpaddr: GPAddr) -> bool {
        self.start <= gpaddr && gpaddr < self.end
    }

    pub fn overlaps(&self, start: GPAddr, end: GPAddr) -> bool {
        self.start < end && start < self.end
    }
}

pub struct Bus {
    regions: Vec<Region>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    pub fn add_device(&mut self, start: GPAddr, size: usize, device: impl Device + 'static) {
        let end = start.checked_add(size).unwrap();

        assert!(
            self.regions
                .iter()
                .all(|region| !region.overlaps(start, end))
        );

        self.regions.push(Region {
            start,
            end,
            device: Box::new(device),
        });
    }

    fn find_mmio_device(&self, gpaddr: GPAddr) -> Result<(&dyn Device, u64), Error> {
        for region in &self.regions {
            if region.contains(gpaddr) {
                let offset = gpaddr.as_usize() - region.start.as_usize();
                return Ok((&*region.device, offset as u64));
            }
        }

        Err(Error::NotMapped)
    }

    pub fn read(
        &self,
        memory: &mut GuestMemory,
        gpaddr: GPAddr,
        value: &mut [u8],
    ) -> Result<(), Error> {
        let (device, offset) = self.find_mmio_device(gpaddr)?;
        device.mmio_read(memory, offset, value)?;
        Ok(())
    }

    pub fn write(
        &self,
        memory: &mut GuestMemory,
        gpaddr: GPAddr,
        value: &[u8],
    ) -> Result<(), Error> {
        let (device, offset) = self.find_mmio_device(gpaddr)?;
        device.mmio_write(memory, offset, value)?;
        Ok(())
    }
}
