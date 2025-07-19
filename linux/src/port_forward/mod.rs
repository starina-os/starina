pub use builder::Builder;
use starina::channel::Channel;
use starina::channel::ChannelReceiver;
use starina::channel::RecvError;
use starina::debug_warn;
use starina::error::ErrorCode;
use starina::handle::Handleable;
use starina::message::Message;
use starina::message::MessageBuffer;
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
            tcpip_rx.handle_id(),
            State::Tcpip(tcpip_rx),
            Readiness::READABLE | Readiness::CLOSED,
        )
        .unwrap();

        for (port, ch) in listen_channels {
            let Port::Tcp {
                guest: guest_port, ..
            } = port;

            poll.add(
                ch.handle_id(),
                State::Listen { ch, guest_port },
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
            sender.send(Message::Data { data }).unwrap();
        });

        let conn_key = self
            .guest_net
            .lock()
            .connect_to_guest(guest_port, IpProto::Tcp, forwarder);

        self.poll
            .add(
                receiver.handle_id(),
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
        let mut msgbuffer = MessageBuffer::new();
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
                            match ch.recv(&mut msgbuffer) {
                                Ok(Message::Connect { ch }) => {
                                    self.new_connection(ch, *guest_port);
                                }
                                Ok(msg) => {
                                    debug_warn!("unexpected message on listen channel: {:?}", msg);
                                }
                                Err(RecvError::Parse(msginfo)) => {
                                    debug_warn!(
                                        "unhandled message type on listen channel: {}",
                                        msginfo.kind()
                                    );
                                }
                                Err(RecvError::Syscall(ErrorCode::Empty)) => {}
                                Err(RecvError::Syscall(err)) => {
                                    debug_warn!("recv error on listen channel: {:?}", err);
                                }
                            }
                        }
                        State::Connected { rx, conn_key }
                            if readiness.contains(Readiness::READABLE) =>
                        {
                            match rx.recv(&mut msgbuffer) {
                                Ok(Message::Data { data }) => {
                                    self.send_tcp_data(memory, conn_key, data);
                                }
                                Ok(msg) => {
                                    debug_warn!(
                                        "unexpected message on connected channel: {:?}",
                                        msg
                                    );
                                }
                                Err(RecvError::Parse(msginfo)) => {
                                    debug_warn!(
                                        "unhandled message type on connected channel: {}",
                                        msginfo.kind()
                                    );
                                }
                                Err(RecvError::Syscall(ErrorCode::Empty)) => {}
                                Err(RecvError::Syscall(err)) => {
                                    debug_warn!("recv error on connected channel: {:?}", err);
                                }
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
