pub use builder::Builder;
use starina::channel::Channel;
use starina::channel::ChannelReceiver;
use starina::handle::Handleable;
use starina::message::Message;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::Box;
use starina::prelude::vec::Vec;
use starina::sync::Arc;
use starina::sync::Mutex;

use crate::guest_memory::GuestMemory;
use crate::guest_net::ConnKey;
use crate::guest_net::GuestNet;
use crate::guest_net::IpProto;
use crate::virtio::device::VirtioMmio;
use crate::virtio::virtio_net::VirtioPacketWriter;

mod builder;

enum State {
    Tcpip(ChannelReceiver),
    Listen {
        ch: Channel,
        guest_port: u16,
    },
    Connected {
        rx: ChannelReceiver,
        conn_key: crate::guest_net::ConnKey,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum Port {
    Tcp { host: u16, guest: u16 },
}

pub struct PortForwarder {
    poll: Poll<State>,
    guest_net: Arc<Mutex<GuestNet>>,
    virtio_net: Arc<VirtioMmio>,
}

impl PortForwarder {
    pub fn new(
        guest_net: Arc<Mutex<GuestNet>>,
        virtio_net: Arc<VirtioMmio>,
        tcpip_rx: ChannelReceiver,
        listen_channels: Vec<(Port, Channel)>,
    ) -> Self {
        let poll = Poll::new().unwrap();

        poll.add(
            tcpip_rx.handle().id(),
            State::Tcpip(tcpip_rx),
            Readiness::READABLE | Readiness::CLOSED,
        )
        .unwrap();

        for (port, handle) in listen_channels {
            let Port::Tcp {
                guest: guest_port, ..
            } = port;

            poll.add(
                handle.handle_id(),
                State::Listen {
                    ch: handle,
                    guest_port,
                },
                Readiness::READABLE,
            )
            .unwrap();
        }

        Self {
            poll,
            guest_net,
            virtio_net,
        }
    }

    pub fn new_connection(&mut self, ch: Channel, guest_port: u16) {
        let (sender, receiver) = ch.split();

        let forwarder = Box::new(move |_conn_key: &ConnKey, data: &[u8]| {
            sender.send(Message::StreamData { data }).unwrap();
        });

        let conn_key = self
            .guest_net
            .lock()
            .connect_to_guest(guest_port, IpProto::Tcp, forwarder);

        self.poll
            .add(
                receiver.handle().id(),
                State::Connected {
                    rx: receiver,
                    conn_key,
                },
                Readiness::READABLE,
            )
            .unwrap();
    }

    pub fn poll(&mut self, memory: &mut GuestMemory) {
        self.process_messages(memory);
        self.flush_pending_packets(memory);
    }

    pub fn process_messages(&mut self, memory: &mut GuestMemory) {
        loop {
            match self.poll.try_wait() {
                Ok((state, readiness)) => {
                    match &*state {
                        State::Tcpip(_ch) => {
                            todo!("unexpected tcpip channel readiness: {:?}", readiness);
                        }
                        State::Listen { ch, guest_port }
                            if readiness.contains(Readiness::READABLE) =>
                        {
                            let mut m = ch.recv().unwrap();
                            if let Some(Message::Connect { handle }) = m.parse() {
                                self.new_connection(handle, *guest_port);
                            }
                        }
                        State::Connected { rx, conn_key }
                            if readiness.contains(Readiness::READABLE) =>
                        {
                            let mut m = rx.recv().unwrap();
                            if let Some(Message::StreamData { data }) = m.parse() {
                                self.send_tcp_data(memory, conn_key, data);
                            }
                        }
                        _ => {}
                    }
                }
                Err(_) => break,
            }
        }
    }

    pub fn flush_pending_packets(&mut self, memory: &mut GuestMemory) {
        self.virtio_net.use_vq(0, |_device, vq| {
            let mut guest_net = self.guest_net.lock();
            while guest_net.has_pending_packets() {
                vq.push_desc(memory, |writer| {
                    let virtio_writer = VirtioPacketWriter::new(writer).unwrap();
                    guest_net.send_pending_packet(virtio_writer)
                })
                .unwrap();
            }
        });
    }

    fn send_tcp_data(&mut self, memory: &mut GuestMemory, conn_key: &ConnKey, data: &[u8]) {
        self.virtio_net.use_vq(0, |_device, vq| {
            vq.push_desc(memory, |writer| {
                let virtio_writer = VirtioPacketWriter::new(writer).unwrap();
                self.guest_net
                    .lock()
                    .send_to_guest(virtio_writer, conn_key, data)
            })
            .unwrap();
        });
    }
}
