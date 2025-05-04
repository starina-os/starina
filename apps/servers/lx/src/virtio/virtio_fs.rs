use super::device::VirtioDevice;

pub struct VirtioFs {}

impl VirtioFs {
    pub fn new() -> Self {
        Self {}
    }
}

impl VirtioDevice for VirtioFs {}
