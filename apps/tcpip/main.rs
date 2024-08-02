#![no_std]
#![no_main]

use ftl_api::channel::Channel;
use ftl_api::channel::ChannelSender;
use ftl_api::collections::HashMap;
use ftl_api::collections::VecDeque;
use ftl_api::handle::OwnedHandle;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_api::types::error::FtlError;
use ftl_api::types::idl::BytesField;
use ftl_api::types::message::HandleOwnership;
use ftl_api::types::message::MessageBuffer;
use ftl_api_autogen::apps::tcpip::Environ;
use ftl_api_autogen::apps::tcpip::Message;
use ftl_api_autogen::protocols::ethernet_device;
use ftl_api_autogen::protocols::tcpip::TcpAccepted;
use ftl_api_autogen::protocols::tcpip::TcpReceived;
use smoltcp::iface::Config;
use smoltcp::iface::Interface;
use smoltcp::iface::SocketHandle;
use smoltcp::iface::SocketSet;
use smoltcp::phy::DeviceCapabilities;
use smoltcp::socket::tcp;
use smoltcp::time::Instant;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::HardwareAddress;
use smoltcp::wire::IpAddress;
use smoltcp::wire::IpCidr;
use smoltcp::wire::IpListenEndpoint;

enum Context {
    Autopilot,
    Driver,
    CtrlSocket,
    DataSocket { handle: SocketHandle },
}

struct RxTokenImpl(Vec<u8>);

impl smoltcp::phy::RxToken for RxTokenImpl {
    fn consume<R, F>(mut self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        f(&mut self.0)
    }
}

struct TxTokenImpl<'a>(&'a mut DeviceImpl);

impl<'a> smoltcp::phy::TxToken for TxTokenImpl<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        trace!("transmitting {} bytes", len);
        let mut buf = [0u8; 1514];
        let ret = f(&mut buf[..len]);

        let tx = ethernet_device::Tx {
            payload: BytesField::new(buf, len.try_into().unwrap()),
        };
        if let Err(err) = self.0.driver_ch.send_with_buffer(&mut self.0.msgbuffer, tx) {
            warn!("failed to send: {:?}", err);
        }

        ret
    }
}

struct DeviceImpl {
    driver_ch: Channel,
    rx_queue: VecDeque<Vec<u8>>,
    msgbuffer: MessageBuffer,
}

impl DeviceImpl {
    pub fn new(driver_ch: Channel) -> DeviceImpl {
        DeviceImpl {
            driver_ch,
            rx_queue: VecDeque::new(),
            msgbuffer: MessageBuffer::new(),
        }
    }

    pub fn receive_pkt(&mut self, pkt: &[u8]) {
        self.rx_queue.push_back(pkt.to_vec());
    }
}

impl smoltcp::phy::Device for DeviceImpl {
    type RxToken<'a> = RxTokenImpl;
    type TxToken<'a> = TxTokenImpl<'a>;

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = smoltcp::phy::Medium::Ethernet;
        caps.max_transmission_unit = 1514;
        caps
    }

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        self.rx_queue
            .pop_front()
            .map(|pkt| (RxTokenImpl(pkt), TxTokenImpl(self)))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(TxTokenImpl(self))
    }
}

fn now() -> Instant {
    // FIXME:
    Instant::from_millis(0)
}

struct Logger;
static LOGGER: Logger = Logger;
impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn flush(&self) {}

    fn log(&self, record: &log::Record) {
        trace!(
            "{}: {}",
            record.module_path().unwrap_or("(unknown)"),
            record.args()
        );
    }
}

struct Server<'a> {
    smol_sockets: SocketSet<'a>,
    device: DeviceImpl,
    iface: Interface,
    sockets: HashMap<SocketHandle, Socket>,
}

impl<'a> Server<'a> {
    pub fn new(driver_ch: Channel, hwaddr: HardwareAddress) -> Server<'a> {
        let config = Config::new(hwaddr.into());
        let mut device = DeviceImpl::new(driver_ch);
        let mut iface = Interface::new(config, &mut device, now());
        let smol_sockets = SocketSet::new(Vec::with_capacity(16));

        // FIXME:
        iface.update_ip_addrs(|ip_addrs| {
            ip_addrs
                .push(IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24))
                .unwrap();
        });

        Server {
            device,
            iface,
            smol_sockets,
            sockets: HashMap::new(),
        }
    }

    pub fn poll(
        &mut self,
        msgbuffer: &mut MessageBuffer,
        mainloop: &mut Mainloop<Context, Message>,
    ) {
        while self
            .iface
            .poll(now(), &mut self.device, &mut self.smol_sockets)
        {
            let mut closed_sockets = Vec::new();
            let mut new_sockets = Vec::new();
            for (handle, sock) in self.sockets.iter_mut() {
                let smol_sock = self.smol_sockets.get_mut::<tcp::Socket>(sock.smol_handle);
                let mut close = false;
                match (&mut sock.state, smol_sock.state()) {
                    (State::Listening { .. }, tcp::State::Listen | tcp::State::SynReceived) => {}
                    (State::Listening { ctrl_sender, listen_endpoint }, tcp::State::Established) => {
                        let (ch1, ch2) = Channel::create().unwrap();

                        // FIXME:
                        let ch1_handle = ch1.handle().id();
                        core::mem::forget(ch1);
                        ctrl_sender
                            .send_with_buffer(
                                msgbuffer,
                                TcpAccepted {
                                    sock: HandleOwnership(ch1_handle),
                                },
                            )
                            .unwrap();

                        // FIXME:
                        let (ch2_sender, ch2_receiver) =
                            Channel::from_handle(OwnedHandle::from_raw(ch2.handle().id())).split();

                        mainloop
                            .add_channel_receiver(
                                ch2_receiver,
                                ch2_sender.clone(),
                                Context::DataSocket {
                                    handle: sock.smol_handle,
                                },
                            )
                            .unwrap();

                        let new_listen_sock_handle = self.smol_sockets.add(tcp::Socket::new(
                            tcp::SocketBuffer::new(vec![0; 8192]),
                            tcp::SocketBuffer::new(vec![0; 8192]),
                        ));
                        let new_listen_sock = self
                            .smol_sockets
                            .get_mut::<tcp::Socket>(new_listen_sock_handle);
                        new_listen_sock
                            .listen(*listen_endpoint)
                            .unwrap();
                        new_sockets.push(Socket {
                            smol_handle: new_listen_sock_handle,
                            state: State::Listening {
                                listen_endpoint: *listen_endpoint,
                                ctrl_sender: ctrl_sender.clone(),
                            },
                        });

                        sock.state = State::Established { sender: ch2_sender };
                    }
                    (State::Listening { .. }, _) => {
                        // Inactive, closed, or unknown state. Close the socket.
                        close = true;
                    }
                    (State::Established { sender: ch }, _) if smol_sock.can_recv() => {
                        loop {
                            let mut buf = [0; 2048];
                            let len = smol_sock.recv_slice(&mut buf).unwrap();
                            if len == 0 {
                                break;
                            }

                            let data = BytesField::new(buf, len.try_into().unwrap());
                            // FIXME: Backpressure
                            ch.send_with_buffer(msgbuffer, TcpReceived { data })
                                .unwrap();
                        }
                    }
                    (State::Established { .. }, tcp::State::Established) => {
                        // Do nothing.
                    }
                    (State::Established { .. }, _) => {
                        close = true;
                    }
                }

                if close {
                    warn!("closing socket");
                    closed_sockets.push(*handle);
                    self.smol_sockets.remove(sock.smol_handle);
                }
            }

            for handle in closed_sockets {
                self.sockets.remove(&handle);
            }

            for socket in new_sockets {
                self.sockets.insert(socket.smol_handle, socket);
            }
        }
    }

    pub fn tcp_listen(&mut self, ctrl_sender: ChannelSender, port: u16) {
        let listen_endpoint = IpListenEndpoint {
            addr: None,
            port,
        };

        let rx_buf = tcp::SocketBuffer::new(vec![0; 8192]);
        let tx_buf = tcp::SocketBuffer::new(vec![0; 8192]);
        let mut sock = tcp::Socket::new(rx_buf, tx_buf);
        sock.listen(listen_endpoint).unwrap();

        info!("listening on port {}", port);
        let handle = self.smol_sockets.add(sock);
        self.sockets.insert(
            handle,
            Socket {
                smol_handle: handle,
                state: State::Listening { ctrl_sender, listen_endpoint },
            },
        );
    }

    pub fn tcp_send(&mut self, handle: SocketHandle, data: &[u8]) -> Result<(), FtlError> {
        let socket = self
            .sockets
            .get_mut(&handle)
            .ok_or(FtlError::HandleNotFound)?;
        if !matches!(socket.state, State::Established { .. }) {
            return Err(FtlError::InvalidState);
        }

        self.smol_sockets
            .get_mut::<tcp::Socket>(socket.smol_handle)
            .send_slice(data)
            .unwrap();
        Ok(())
    }

    pub fn receive_pkt(&mut self, pkt: &[u8]) {
        self.device.receive_pkt(pkt);
    }
}

enum State {
    Listening { ctrl_sender: ChannelSender, listen_endpoint: IpListenEndpoint },
    Established { sender: ChannelSender },
}

struct Socket {
    smol_handle: SocketHandle,
    state: State,
}

#[ftl_api::main]
pub fn main(mut env: Environ) {
    info!("starting...");

    // For smoltcp
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(if cfg!(debug_assertions) {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    });

    let driver_ch = env.depends.ethernet_device.take().unwrap();
    // FIXME: Clone using syscall
    let driver_ch_cloned = Channel::from_handle(OwnedHandle::from_raw(driver_ch.handle().id()));

    let mac = HardwareAddress::Ethernet(EthernetAddress([0x52, 0x54, 0x00, 0x12, 0x34, 0x56])); // FIXME:
    let mut server = Server::new(driver_ch_cloned, mac);

    let mut mainloop = Mainloop::<Context, Message>::new().unwrap();
    mainloop
        .add_channel(env.autopilot_ch.take().unwrap(), Context::Autopilot)
        .unwrap();
    mainloop.add_channel(driver_ch, Context::Driver).unwrap();

    let mut buffer = MessageBuffer::new();
    loop {
        server.poll(&mut buffer, &mut mainloop);
        match mainloop.next(&mut buffer) {
            Event::Message { sender, ctx, m } => {
                match (ctx, m) {
                    (Context::Autopilot, Message::NewclientRequest(m)) => {
                        info!("got new client: {:?}", m.handle());
                        let new_ch = Channel::from_handle(OwnedHandle::from_raw(m.handle()));
                        mainloop.add_channel(new_ch, Context::CtrlSocket).unwrap();
                    }
                    (Context::CtrlSocket, Message::TcpListenRequest(m)) => {
                        server.tcp_listen(sender.clone(), m.port());
                    }
                    (Context::DataSocket { handle }, Message::TcpSendRequest(m)) => {
                        server.tcp_send(*handle, m.data().as_slice()).unwrap();
                    }
                    (Context::Driver, Message::Rx(m)) => {
                        trace!(
                            "received {} bytes: {:02x?}",
                            m.payload().len(),
                            &m.payload().as_slice()[0..14]
                        );
                        server.receive_pkt(m.payload().as_slice());
                    }
                    _ => {
                        // TODO: dump message with fmt::Debug
                        panic!("unknown message");
                    }
                }
            }
            _ => {
                panic!("unexpected event");
            }
        }
    }
}
