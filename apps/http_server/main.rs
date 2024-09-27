#![no_std]
#![no_main]

ftl_api::autogen!();

use ftl_api::channel::Channel;
use ftl_api::environ::Environ;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_api::types::message::MessageBuffer;
use ftl_autogen::idl::tcpip::TcpListen;
use ftl_autogen::idl::Message;

mod http;

#[derive(Debug)]
enum Context {
    Startup,
    // TCP/IP listen channel (so-called TCP backlog).
    Listen,
    // TCP/IP data channel. Represents each TCP connection.
    Conn(http::Conn),
}

#[no_mangle]
pub fn main(mut env: Environ) {
    info!("starting");
    let startup_ch = env.take_channel("dep:startup").unwrap();
    let tcpip_ch = env.take_channel("dep:tcpip").unwrap();

    let mut msgbuffer = MessageBuffer::new();
    let listen_reply = tcpip_ch
        .call(TcpListen { port: 80 }, &mut msgbuffer)
        .unwrap();
    let listen_ch = listen_reply.listen.take::<Channel>().unwrap();

    let mut mainloop = Mainloop::<Context, Message>::new().unwrap();
    mainloop.add_channel(startup_ch, Context::Startup).unwrap();
    mainloop.add_channel(listen_ch, Context::Listen).unwrap();

    loop {
        match mainloop.next() {
            Event::Message {
                ctx: Context::Listen,
                message: Message::TcpAccepted(m),
                ..
            } => {
                let sock_ch = m.conn.take::<Channel>().unwrap();
                mainloop
                    .add_channel(sock_ch, Context::Conn(http::Conn::new()))
                    .unwrap();
            }
            Event::Message {
                ctx: Context::Conn(conn),
                message: Message::TcpReceived(m),
                sender,
            } => {
                conn.tcp_receive(m.data.as_slice(), sender);
            }
            Event::Message {
                ctx: Context::Conn(_),
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
