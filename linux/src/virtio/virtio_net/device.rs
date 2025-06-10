use starina::prelude::*;
use starina::sync::Mutex;
use starina_utils::endianness::LittleEndian;

use crate::guest_memory::GuestMemory;
use crate::guest_net::ConnKey;
use crate::guest_net::GuestNet;
use crate::guest_net::MacAddr;
use crate::virtio::device::VirtioDevice;
use crate::virtio::virtqueue::DescChain;
use crate::virtio::virtqueue::DescChainReader;
use crate::virtio::virtqueue::Virtqueue;

const VIRTIO_NET_F_MAC: u64 = 1 << 5;

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
    guest_mac: MacAddr,
}

impl VirtioNet {
    pub fn new(guest_net: GuestNet, guest_mac: MacAddr) -> Self {
        Self {
            guest_net: Mutex::new(guest_net),
            guest_mac,
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

        let mut guest_net = self.guest_net.lock();
        match guest_net.recv_from_guest(reader) {
            Ok(_) => {}
            Err(err) => {
                debug_warn!("virtio-net: recv_from_guest: {:?}", err);
            }
        }
    }

    pub fn do_send_to_guest(
        &self,
        memory: &mut GuestMemory,
        vq: &mut Virtqueue,
        conn: &ConnKey,
        payload: &[u8],
    ) {
        // Check if connection exists, if not, establish it
        {
            let mut guest_net = self.guest_net.lock();
            let connection_exists = guest_net.has_connection(conn);

            if !connection_exists {
                info!("Establishing new TCP connection for {:?}", conn);

                // Get a descriptor for sending SYN
                let Some(desc) = vq.pop_avail(memory) else {
                    debug_warn!("virtio-net: no available descriptor for SYN");
                    return;
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

                // Initiate TCP connection
                match guest_net.connect_to_guest(writer, *conn) {
                    Ok(Some(written_len)) => {
                        info!(
                            "SYN sent for connection {:?}, written {} bytes",
                            conn, written_len
                        );
                        vq.push_used(memory, desc, written_len as u32); // SYN packet sent
                    }
                    Ok(None) => {
                        // TODO: push back to available queue
                        return;
                    }
                    Err(e) => {
                        debug_warn!("Failed to send SYN: {:?}", e);
                        // TODO: push back to available queue
                        return;
                    }
                }
            }
        }

        // Now send the actual data
        let Some(desc) = vq.pop_avail(memory) else {
            debug_warn!("virtio-net: send_to_guest: no available descriptor");
            return;
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

        match self.guest_net.lock().send_to_guest(writer, conn, payload) {
            Ok(Some(written_len)) => vq.push_used(memory, desc, written_len as u32),
            Ok(None) => {
                // TODO: push back to available queue
            }
            Err(e) => {
                panic!("virtio-net: send_to_guest: {:?}", e);
            }
        }
    }
}

impl VirtioDevice for VirtioNet {
    fn connect_to_guest(&self, connkey: ConnKey) {
        // Connection will be initiated when first packet is sent
        // This is because we need a writer (virtqueue) to send the SYN packet
        info!(
            "Connection requested for {:?}, will establish on first send",
            connkey
        );
    }

    fn flush_arp_reply(&self, memory: &mut GuestMemory, vq: &mut Virtqueue) {
        // TODO: refactor
        let mut guest_net = self.guest_net.lock();
        if guest_net.needs_reply_host_arp_request() {
            let chain = vq.pop_avail(memory).unwrap();
            let (_, mut writer) = chain.split(vq, memory).unwrap();
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

            let written_len = guest_net.reply_host_arp_request(writer).unwrap();
            vq.push_used(memory, chain, written_len as u32);
        }
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
