use starina::channel::Channel;
use starina::collections::HashMap;
use starina::message::CallId;
use starina::message::Message;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::format;
use starina::prelude::vec::Vec;
use starina::sync::Arc;
use starina::sync::Mutex;

use super::Port;
use super::PortForwarder;
use crate::guest_net::GuestNet;
use crate::virtio::device::VirtioMmio;

pub struct Builder<'a> {
    tcpip_ch: Channel,
    ports: &'a [Port],
    guest_net: Arc<Mutex<GuestNet>>,
    virtio_net: Arc<VirtioMmio>,
}

impl<'a> Builder<'a> {
    pub fn new(
        tcpip_ch: Channel,
        guest_net: Arc<Mutex<GuestNet>>,
        virtio_net: Arc<VirtioMmio>,
        ports: &'a [Port],
    ) -> Self {
        Self {
            tcpip_ch,
            ports,
            guest_net,
            virtio_net,
        }
    }

    pub fn build(self) -> PortForwarder {
        let guest_net = self.guest_net;
        let virtio_net = self.virtio_net;

        let (tcpip_tx, tcpip_rx) = self.tcpip_ch.split();
        let poll = Poll::new().unwrap();

        poll.add(
            tcpip_rx.handle().id(),
            (),
            Readiness::READABLE | Readiness::CLOSED,
        )
        .unwrap();

        // Ask the tcpip to create listen channels.
        let mut remaining = HashMap::with_capacity(self.ports.len());
        for (index, port) in self.ports.iter().enumerate() {
            let Port::Tcp { host, .. } = port;
            let call_id = CallId::from(index as u32);
            let uri = format!("tcp-listen:0.0.0.0:{}", host);

            tcpip_tx
                .send(Message::Open {
                    call_id,
                    uri: uri.as_bytes(),
                })
                .unwrap();

            remaining.insert(call_id, port);
        }

        // Wait for all the listen channels to be opened.
        let mut listen_channels = Vec::new();
        while !remaining.is_empty() {
            let (_, readiness) = poll.wait().unwrap();
            if readiness.contains(Readiness::READABLE) {
                let mut m = tcpip_rx.recv().unwrap();
                if let Some(Message::OpenReply { call_id, handle }) = m.parse() {
                    let port = remaining.remove(&call_id).unwrap();
                    listen_channels.push((*port, handle));
                }
            }
        }

        PortForwarder::new(guest_net, virtio_net, tcpip_rx, listen_channels)
    }
}
