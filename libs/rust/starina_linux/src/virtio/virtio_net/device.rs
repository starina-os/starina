use starina::prelude::*;
use starina_utils::endianness::LittleEndian;

use crate::guest_memory::GuestMemory;
use crate::virtio::device::VirtioDevice;
use crate::virtio::virtqueue::DescChain;
use crate::virtio::virtqueue::Virtqueue;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct VirtioNetHdr {
    flags: u8,
    gso_type: u8,
    hdr_len: LittleEndian<u16>,
    gso_size: LittleEndian<u16>,
    csum_start: LittleEndian<u16>,
    csum_offset: LittleEndian<u16>,
    num_buffers: LittleEndian<u16>,
}

pub struct VirtioNet {}

impl VirtioNet {
    pub fn new() -> Self {
        Self {}
    }

    /// Processes a host-to-guest packet.
    fn process_rx(&self, memory: &mut GuestMemory, vq: &mut Virtqueue, chain: DescChain) {
        todo!()
    }

    /// Processes a guest-to-host packet.
    fn process_tx(&self, memory: &mut GuestMemory, vq: &mut Virtqueue, chain: DescChain) {
        let (mut reader, writer) = chain.reader_writer(vq, memory).unwrap();
        let header = match reader.read::<VirtioNetHdr>() {
            Ok(header) => header,
            Err(e) => {
                debug_warn!("failed to read virtio-net header: {:?}", e);
                return;
            }
        };

        info!("virtio-net tx header: {:x?}", header);

        // We don't support any flags yet.
        assert_eq!(header.flags, 0);

        let hdr_len: u16 = header.hdr_len.to_host();
        let packet = match reader.read_zerocopy2() {
            Ok(packet) => packet,
            Err(e) => {
                debug_warn!("failed to read virtio-net packet: {:?}", e);
                return;
            }
        };

        trace!("virtio-net tx packet: {:x?}", packet);
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
        match vq.index() {
            0 => self.process_rx(memory, vq, chain),
            1 => self.process_tx(memory, vq, chain),
            i => panic!("unexpected virtio-net queue index: {}", i),
        }
    }
}
