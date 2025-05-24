use super::device::VirtioDevice;

pub struct VirtioNet {}

impl VirtioNet {
    pub fn new() -> Self {
        Self {}
    }
}

impl VirtioDevice for VirtioNet {
    fn num_queues(&self) -> u32 {
        2 /* RX and TX queues */
    }

    fn device_features(&self) -> u64 {
        todo!()
    }

    fn device_id(&self) -> u32 {
        todo!()
    }

    fn vendor_id(&self) -> u32 {
        todo!()
    }

    fn config_read(&self, offset: u64, buf: &mut [u8]) {
        todo!()
    }

    fn process(
        &self,
        memory: &mut crate::guest_memory::GuestMemory,
        vq: &mut super::virtqueue::Virtqueue,
        chain: super::virtqueue::DescChain,
    ) {
        todo!()
    }
}
