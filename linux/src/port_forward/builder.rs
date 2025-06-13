use starina::channel::Channel;
use starina::channel::ChannelReceiver;
use starina::collections::HashMap;
use starina::handle::Handleable;
use starina::message::CallId;
use starina::message::Message;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::format;
use starina::prelude::Vec;
use starina::sync::Arc;
use starina::sync::Mutex;

use crate::Port;
use crate::guest_net::GuestNet;
use crate::virtio::device::VirtioMmio;

use super::PortForwarder;

pub enum BuilderState {
    Tcpip(ChannelReceiver),
    Listen { ch: Channel, guest_port: u16 },
    Connected { rx: ChannelReceiver, conn_key: crate::guest_net::ConnKey },
}

pub struct PortForwarderBuilder {
    tcpip_ch: Channel,
    ports: Vec<Port>,
    guest_net: Option<Arc<Mutex<GuestNet>>>,
    virtio_mmio_net: Option<Arc<VirtioMmio>>,
}

impl PortForwarderBuilder {
    pub fn new(tcpip_ch: Channel) -> Self {
        Self {
            tcpip_ch,
            ports: Vec::new(),
            guest_net: None,
            virtio_mmio_net: None,
        }
    }

    pub fn with_ports(mut self, ports: &[Port]) -> Self {
        self.ports.extend_from_slice(ports);
        self
    }

    pub fn with_guest_net(mut self, guest_net: Arc<Mutex<GuestNet>>) -> Self {
        self.guest_net = Some(guest_net);
        self
    }

    pub fn with_virtio_net(mut self, virtio_mmio_net: Arc<VirtioMmio>) -> Self {
        self.virtio_mmio_net = Some(virtio_mmio_net);
        self
    }

    pub fn build(self) -> PortForwarder {
        let guest_net = self.guest_net.expect("guest_net must be set");
        let virtio_mmio_net = self.virtio_mmio_net.expect("virtio_mmio_net must be set");
        
        let (tcpip_tx, tcpip_rx) = self.tcpip_ch.split();
        let poll = Poll::new().unwrap();
        
        poll.add(
            tcpip_rx.handle().id(),
            BuilderState::Tcpip(tcpip_rx),
            Readiness::READABLE | Readiness::CLOSED,
        ).unwrap();

        let mut remaining = HashMap::with_capacity(self.ports.len());
        
        for (index, port) in self.ports.iter().enumerate() {
            let Port::Tcp { host, .. } = port;
            let call_id = CallId::from(index as u32);
            let uri = format!("tcp-listen:0.0.0.0:{}", host);
            
            tcpip_tx.send(Message::Open {
                call_id,
                uri: uri.as_bytes(),
            }).unwrap();
            
            remaining.insert(call_id, port);
        }

        while !remaining.is_empty() {
            let (state, readiness) = poll.wait().unwrap();
            
            if let BuilderState::Tcpip(ch) = &*state {
                if readiness.contains(Readiness::READABLE) {
                    let mut m = ch.recv().unwrap();
                    
                    if let Some(Message::OpenReply { call_id, handle }) = m.parse() {
                        let port = remaining.remove(&call_id).unwrap();
                        let Port::Tcp { guest: guest_port, .. } = port;
                        
                        poll.add(
                            handle.handle_id(),
                            BuilderState::Listen { ch: handle, guest_port: *guest_port },
                            Readiness::READABLE,
                        ).unwrap();
                    }
                }
            }
        }
        
        PortForwarder::new(poll, guest_net, virtio_mmio_net)
    }
}