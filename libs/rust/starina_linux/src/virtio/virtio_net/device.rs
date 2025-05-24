use crate::guest_memory::GuestMemory;
use crate::virtio::device::VirtioDevice;
use crate::virtio::virtqueue::DescChain;
use crate::virtio::virtqueue::Virtqueue;

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
        0
    }

    fn device_id(&self) -> u32 {
        1 /* virtio-net */
    }

    fn vendor_id(&self) -> u32 {
        0
    }

    fn config_read(&self, offset: u64, buf: &mut [u8]) {
        todo!()
    }

    fn process(&self, memory: &mut GuestMemory, vq: &mut Virtqueue, chain: DescChain) {
        todo!()
    }
}
