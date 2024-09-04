#![no_std]
#![no_main]

use ftl_api::channel::ChannelSender;
use ftl_api::environ::Environ;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_api_autogen::apps::http_server::Message;
use ftl_api_autogen::protocols::tcpip::TcpListenRequest;
use ftl_api_autogen::protocols::tcpip::TcpSendRequest;

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
        tcp_sender.send(TcpSendRequest { data }).unwrap();
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
    tcpip_ch.send(TcpListenRequest { port: 80 }).unwrap();

    let mut mainloop = Mainloop::<Context, Message>::new().unwrap();
    mainloop
        .add_channel(env.take_channel("dep:startup").unwrap(), Context::Startup)
        .unwrap();
    mainloop.add_channel(tcpip_ch, Context::Ctrl).unwrap();

    loop {
        match mainloop.next() {
            Event::Message(Context::Ctrl, Message::TcpAccepted(mut m), _) => {
                let sock_ch = m.sock().unwrap();
                mainloop
                    .add_channel(sock_ch, Context::Data(Client::new()))
                    .unwrap();
            }
            Event::Message(Context::Data(client), Message::TcpReceived(m), sender) => {
                client.receive(sender, m.data().as_slice());
            }
            Event::Message(Context::Data(_), Message::TcpClosed(_), sender) => {
                trace!("client connection closed");
                let sender_id = sender.handle().id();
                mainloop.remove(sender_id);
            }
            ev => {
                warn!("unexpected event: {:?}", ev);
            }
        }
    }
}
