use starina::prelude::*;
use starina_utils::endianness::LittleEndian;

use crate::guest_memory::GuestMemory;
use crate::guest_net::MacAddr;
use crate::virtio::device::VirtioDevice;
use crate::virtio::virtqueue::DescChain;
use crate::virtio::virtqueue::DescChainReader;
use crate::virtio::virtqueue::DescChainWriter;
use crate::virtio::virtqueue::Virtqueue;

const VIRTIO_NET_F_MAC: u64 = 1 << 5;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VirtioNetHdr {
    flags: u8,
    gso_type: u8,
    hdr_len: LittleEndian<u16>,
    gso_size: LittleEndian<u16>,
    csum_start: LittleEndian<u16>,
    csum_offset: LittleEndian<u16>,
    num_buffers: LittleEndian<u16>,
}

pub fn prepare_tx_packet_writer<F, E>(
    memory: &mut GuestMemory,
    vq: &mut Virtqueue,
    f: F,
) -> Result<(), ()>
where
    F: FnOnce(DescChainWriter<'_>) -> Result<Option<usize>, E>,
{
    // Now send the actual data
    let Some(desc) = vq.pop_avail(memory) else {
        debug_warn!("virtio-net: send_to_guest: no available descriptor");
        return Err(());
    };

    let (_, mut writer) = desc.split(vq, memory).unwrap();
    writer
        .write(VirtioNetHdr {
            flags: 0,
            gso_type: 0,
            hdr_len: 0.into(),
            gso_size: 0.into(),
            csum_start: 0.into(),
            csum_offset: 0.into(),
            num_buffers: 1.into(),
        })
        .unwrap();

    let result = f(writer);

    match result {
        Ok(Some(written_len)) => vq.push_used(memory, desc, written_len as u32),
        Ok(None) => {
            // TODO: push back to available queue
        }
        Err(_) => {
            panic!("virtio-net: send_to_guest");
        }
    }

    Ok(())
}

pub struct VirtioNet {
    guest_mac: MacAddr,
    receiver: Box<dyn for<'a> Fn(DescChainReader<'a>)>,
}

impl VirtioNet {
    pub fn new(guest_mac: MacAddr, receiver: Box<dyn for<'a> Fn(DescChainReader<'a>)>) -> Self {
        Self {
            guest_mac,
            receiver,
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

        (self.receiver)(reader);
    }
}

impl VirtioDevice for VirtioNet {
    fn num_queues(&self) -> u32 {
        2 /* RX and TX queues */
    }

    fn device_features(&self) -> u64 {
        VIRTIO_NET_F_MAC
    }

    fn device_id(&self) -> u32 {
        1 /* virtio-net */
    }

    fn vendor_id(&self) -> u32 {
        0
    }

    fn config_read(&self, offset: u64, buf: &mut [u8]) {
        match offset {
            0..=5 => {
                // MAC address at offset 0-5
                let mac_bytes: [u8; 6] = self.guest_mac.into();
                let start = offset as usize;
                let end = core::cmp::min(start + buf.len(), 6);
                if start < 6 {
                    let copy_len = end - start;
                    buf[..copy_len].copy_from_slice(&mac_bytes[start..end]);
                }
            }
            _ => {
                todo!("virtio-net: config_read: unknown offset: {}", offset);
            }
        }
    }

    fn process(&self, memory: &mut GuestMemory, vq: &mut Virtqueue, chain: DescChain) {
        match vq.index() {
            0 => {
                // receiveq: Do nothing.
            }
            1 => {
                let (reader, _) = chain.split(vq, memory).unwrap();
                self.process_tx(reader);
                vq.push_used(memory, chain, 0);
            }
            i => panic!("unexpected virtio-net queue index: {}", i),
        }
    }
}
