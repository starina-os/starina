use starina::collections::HashMap;
use starina::prelude::*;
use starina::sync::Arc;

use crate::guest_memory::GuestMemory;
use crate::virtio::device::VirtioMmio;
use crate::virtio::virtio_net::VirtioNet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ConnKey {
    pub protocol: Protocol,
    pub host_port: u16,
    pub guest_port: u16,
}

struct Conn {}

pub struct GuestNet {
    device: Arc<VirtioMmio>,
    connections: HashMap<ConnKey, Conn>,
}

impl GuestNet {
    pub fn new(device: Arc<VirtioMmio>) -> Self {
        Self {
            device,
            connections: HashMap::new(),
        }
    }

    /// Writes TCP/UDP payload to the guest.
    pub fn send_to_guest(&self, memory: &mut GuestMemory, key: &ConnKey, data: &[u8]) {
        let Some(conn) = self.connections.get(key) else {
            debug_warn!("unknown network connection: {:?}", key);
            return;
        };

        const VIRTIO_NET_RECEIVEQ: u32 = 0;
        self.device.use_vq(VIRTIO_NET_RECEIVEQ, |vq| {
            let Some(desc) = vq.pop_avail(memory) else {
                debug_warn!("guest's receiveq is empty");
                return;
            };

            let (_, mut writer) = desc.split(vq, memory).unwrap();
            // writer.write(eth_header).unwrap();
            // writer.write(ip_header).unwrap();
            // writer.write(tcp_udp_header).unwrap();
            writer.write_bytes(data).unwrap();
            vq.push_used(memory, desc, data.len() as u32);
        });
    }
}
