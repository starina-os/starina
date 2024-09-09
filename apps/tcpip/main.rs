#![no_std]
#![no_main]

ftl_api::autogen!();

use ftl_api::channel::Channel;
use ftl_api::channel::ChannelSender;
use ftl_api::collections::HashMap;
use ftl_api::collections::VecDeque;
use ftl_api::environ::Environ;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_api::types::error::FtlError;
use ftl_autogen::idl::ethernet_device;
use ftl_autogen::idl::tcpip::TcpAccepted;
use ftl_autogen::idl::tcpip::TcpClosed;
use ftl_autogen::idl::tcpip::TcpReceived;
use ftl_autogen::idl::Message;
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

#[derive(Debug)]
enum Context {
    Startup,
    Driver,
    CtrlSocket,
    DataSocket(SocketHandle),
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
            payload: buf[..len].try_into().unwrap(),
        };
        if let Err(err) = self.0.driver_sender.send(tx) {
            warn!("failed to send: {:?}", err);
        }

        ret
    }
}

struct DeviceImpl {
    driver_sender: ChannelSender,
    rx_queue: VecDeque<Vec<u8>>,
}

impl DeviceImpl {
    pub fn new(driver_sender: ChannelSender) -> DeviceImpl {
        DeviceImpl {
            driver_sender,
            rx_queue: VecDeque::new(),
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
    pub fn new(driver_sender: ChannelSender, hwaddr: HardwareAddress) -> Server<'a> {
        let config = Config::new(hwaddr.into());
        let mut device = DeviceImpl::new(driver_sender);
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

    pub fn poll(&mut self, mainloop: &mut Mainloop<Context, Message>) {
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
                    (
                        State::Listening {
                            ctrl_sender,
                            listen_endpoint,
                        },
                        tcp::State::Established,
                    ) => {
                        let (ch1, ch2) = Channel::create().unwrap();
                        ctrl_sender.send(TcpAccepted { sock: ch1.into() }).unwrap();

                        let (ch2_sender, ch2_receiver) = ch2.split();
                        mainloop
                            .add_channel(
                                (ch2_sender.clone(), ch2_receiver),
                                Context::DataSocket(sock.smol_handle),
                            )
                            .unwrap();

                        let new_listen_sock_handle = self.smol_sockets.add(tcp::Socket::new(
                            tcp::SocketBuffer::new(vec![0; 8192]),
                            tcp::SocketBuffer::new(vec![0; 8192]),
                        ));
                        let new_listen_sock = self
                            .smol_sockets
                            .get_mut::<tcp::Socket>(new_listen_sock_handle);
                        new_listen_sock.listen(*listen_endpoint).unwrap();
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

                            // FIXME: Backpressure
                            ch.send(TcpReceived {
                                data: buf[..len].try_into().unwrap(),
                            })
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
                }
            }

            for handle in closed_sockets {
                let sock = self.sockets.remove(&handle).unwrap();
                self.smol_sockets.remove(sock.smol_handle);

                match sock.state {
                    State::Listening {
                        ctrl_sender: sender,
                        ..
                    }
                    | State::Established { sender } => {
                        sender.send(TcpClosed {}).unwrap();
                        mainloop.remove(sender.handle().id());
                    }
                }
            }

            for socket in new_sockets {
                self.sockets.insert(socket.smol_handle, socket);
            }
        }
    }

    pub fn tcp_listen(&mut self, ctrl_sender: ChannelSender, port: u16) {
        let listen_endpoint = IpListenEndpoint { addr: None, port };

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
                state: State::Listening {
                    ctrl_sender,
                    listen_endpoint,
                },
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
    Listening {
        ctrl_sender: ChannelSender,
        listen_endpoint: IpListenEndpoint,
    },
    Established {
        sender: ChannelSender,
    },
}

struct Socket {
    smol_handle: SocketHandle,
    state: State,
}

#[no_mangle]
pub fn main(mut env: Environ) {
    info!("starting...");

    // For smoltcp
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(if cfg!(debug_assertions) {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    });

    let driver_ch = env.take_channel("dep:ethernet_device").unwrap();
    let startup_ch = env.take_channel("dep:startup").unwrap();
    let (driver_sender, driver_receiver) = driver_ch.split();

    let mac = HardwareAddress::Ethernet(EthernetAddress([0x52, 0x54, 0x00, 0x12, 0x34, 0x56])); // FIXME:
    let mut server = Server::new(driver_sender.clone(), mac);

    let mut mainloop = Mainloop::<Context, Message>::new().unwrap();
    mainloop.add_channel(startup_ch, Context::Startup).unwrap();
    mainloop
        .add_channel((driver_sender, driver_receiver), Context::Driver)
        .unwrap();

    loop {
        server.poll(&mut mainloop);
        match mainloop.next() {
            Event::Message {
                ctx: Context::Startup,
                message: Message::NewClient(m),
                ..
            } => {
                info!("new autopilot msg...");
                let new_ch = m.handle.take::<Channel>().unwrap();
                info!("got new client: {:?}", new_ch);
                mainloop.add_channel(new_ch, Context::CtrlSocket).unwrap();
            }
            Event::Message {
                ctx: Context::CtrlSocket,
                message: Message::TcpListen(m),
                sender,
            } => {
                server.tcp_listen(sender.clone(), m.port);
            }
            Event::Message {
                ctx: Context::DataSocket(handle),
                message: Message::TcpSend(m),
                ..
            } => {
                server.tcp_send(*handle, m.data.as_slice()).unwrap();
            }
            Event::Message {
                ctx: Context::Driver,
                message: Message::Rx(m),
                ..
            } => {
                trace!(
                    "received {} bytes: {:02x?}",
                    m.payload.len(),
                    &m.payload.as_slice()[0..14]
                );
                server.receive_pkt(m.payload.as_slice());
            }
            ev => {
                warn!("unhandled event: {:?}", ev);
            }
        }
    }
}
