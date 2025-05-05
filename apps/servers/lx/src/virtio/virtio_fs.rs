use super::device::VirtioDevice;

pub struct VirtioFs {}

impl VirtioFs {
    pub fn new() -> Self {
        Self {}
    }
}

impl VirtioDevice for VirtioFs {
    fn num_queues(&self) -> u32 {
        3
    }

    fn device_features(&self) -> u64 {
        0
    }

    fn driver_features(&self) -> u64 {
        0
    }

    fn device_id(&self) -> u32 {
        26
    }

    fn vendor_id(&self) -> u32 {
        0
    }
}
