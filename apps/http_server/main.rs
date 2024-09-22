#![no_std]
#![no_main]

ftl_api::autogen!();

use ftl_api::channel::Channel;
use ftl_api::channel::ChannelSender;
use ftl_api::environ::Environ;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_autogen::idl::tcpip::TcpListen;
use ftl_autogen::idl::tcpip::TcpSend;
use ftl_autogen::idl::Message;

#[derive(Debug)]
struct Client {
    buffered: Vec<u8>,
}

impl Client {
    pub fn new() -> Client {
        Client {
            buffered: Vec::new(),
        }
    }

    fn handle_request(&self, tcp_sender: &ChannelSender, req: httparse::Request<'_, '_>) {
        trace!("request: {:?}", req);

        let data = &b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\nHello, world!"[..];
        tcp_sender
            .send(TcpSend {
                data: data.try_into().unwrap(),
            })
            .unwrap();
    }

    pub fn receive(&mut self, tcp_sender: &ChannelSender, data: &[u8]) {
        self.buffered.extend_from_slice(&data);

        if let Some(index) = self.buffered.windows(4).position(|w| w == b"\r\n\r\n") {
            let request_bytes = &self.buffered[..index + 4];

            let mut headers = [httparse::EMPTY_HEADER; 32];
            let mut req = httparse::Request::new(&mut headers);
            match req.parse(request_bytes) {
                Ok(httparse::Status::Complete(_len)) => {
                    self.handle_request(tcp_sender, req);
                    self.buffered = Vec::new();
                }
                Ok(httparse::Status::Partial) => {
                    warn!("partial request");
                    return;
                }
                Err(e) => {
                    warn!("error parsing request: {:?}", e);
                    return;
                }
            };
        }
    }
}

#[derive(Debug)]
enum Context {
    Startup,
    // TCP/IP control channel.
    Ctrl,
    // TCP/IP data channel. Represents each TCP connection.
    Data(Client),
}

#[no_mangle]
pub fn main(mut env: Environ) {
    info!("starting");
    let tcpip_ch = env.take_channel("dep:tcpip").unwrap();

    // FIXME: Wait for the reply to make sure it succeeds.
    tcpip_ch.send(TcpListen { port: 80 }).unwrap();

    let mut mainloop = Mainloop::<Context, Message>::new().unwrap();
    mainloop
        .add_channel(env.take_channel("dep:startup").unwrap(), Context::Startup)
        .unwrap();
    mainloop.add_channel(tcpip_ch, Context::Ctrl).unwrap();

    loop {
        match mainloop.next() {
            Event::Message {
                ctx: Context::Ctrl,
                message: Message::TcpAccepted(m),
                ..
            } => {
                let sock_ch = m.sock.take::<Channel>().unwrap();
                mainloop
                    .add_channel(sock_ch, Context::Data(Client::new()))
                    .unwrap();
            }
            Event::Message {
                ctx: Context::Data(client),
                message: Message::TcpReceived(m),
                sender,
            } => {
                client.receive(sender, m.data.as_slice());
            }
            Event::Message {
                ctx: Context::Data(_),
                message: Message::TcpClosed(_),
                sender,
            } => {
                trace!("client connection closed");
                let sender_id = sender.handle().id();
                mainloop.remove(sender_id).unwrap();
            }
            ev => {
                warn!("unexpected event: {:?}", ev);
            }
        }
    }
}
