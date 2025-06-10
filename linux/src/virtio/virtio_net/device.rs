use starina::prelude::*;
use starina::sync::Mutex;
use starina_utils::endianness::LittleEndian;

use crate::guest_memory::GuestMemory;
use crate::guest_net::ConnKey;
use crate::guest_net::GuestNet;
use crate::virtio::device::VirtioDevice;
use crate::virtio::virtqueue::DescChain;
use crate::virtio::virtqueue::DescChainReader;
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

pub struct VirtioNet {
    guest_net: Mutex<GuestNet>,
}

impl VirtioNet {
    pub fn new(guest_net: GuestNet) -> Self {
        Self {
            guest_net: Mutex::new(guest_net),
        }
    }

    /// Processes a guest-to-host packet.
    fn process_tx(&self, mut reader: DescChainReader<'_>) {
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

        if let Err(err) = self.guest_net.lock().recv_from_guest(reader) {
            debug_warn!("virtio-net: recv_from_guest: {:?}", err);
        }
    }

    pub fn do_send_to_guest(
        &self,
        memory: &mut GuestMemory,
        vq: &mut Virtqueue,
        conn: &ConnKey,
        payload: &[u8],
    ) {
        let Some(desc) = vq.pop_avail(memory) else {
            debug_warn!("virtio-net: send_to_guest: no available descriptor");
            return;
        };

        let (_, writer) = desc.split(vq, memory).unwrap();
        match self.guest_net.lock().send_to_guest(writer, conn, payload) {
            Ok(_) => vq.push_used(memory, desc, payload.len() as u32),
            Err(e) => {
                panic!("virtio-net: send_to_guest: {:?}", e);
                // TODO: push back to available queue
            }
        }
    }
}

impl VirtioDevice for VirtioNet {
    fn connect_to_guest(&self, connkey: ConnKey) {
        self.guest_net.lock().connect_to_guest(connkey);
    }

    fn send_to_guest(
        &self,
        memory: &mut GuestMemory,
        vq: &mut Virtqueue,
        connkey: &ConnKey,
        payload: &[u8],
    ) {
        self.do_send_to_guest(memory, vq, connkey, payload);
    }

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

    fn config_read(&self, _offset: u64, _buf: &mut [u8]) {
        todo!()
    }

    fn process(&self, memory: &mut GuestMemory, vq: &mut Virtqueue, chain: DescChain) {
        match vq.index() {
            0 => {
                // receiveq: Do nothing.
            }
            1 => {
                let (reader, _) = chain.split(vq, memory).unwrap();
                self.process_tx(reader);
                // FIXME: VIRQ_IRQSTATUS_QUEUE causes a hang.
                // vq.push_used(memory, chain, 0);
            }
            i => panic!("unexpected virtio-net queue index: {}", i),
        }
    }
}
