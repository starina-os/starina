use ftl_api::channel::ChannelSender;
use ftl_api::prelude::*;
use httparse::Request;
use httparse::Status;
use httparse::EMPTY_HEADER;

use crate::ftl_autogen::idl::tcpip::TcpSend;

const REQUEST_MAX_SIZE: usize = 32 * 1024;

fn do_handle_request(req: Request, tcp_sender: &ChannelSender) {
    info!("{}, {}", req.method.unwrap(), req.path.unwrap());

    let data = b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\nHello, world!";
    tcp_sender
        .send(TcpSend {
            data: data.as_slice().try_into().unwrap(),
        })
        .unwrap();
}

fn handle_request(buf: &[u8], tcp_sender: &ChannelSender) {
    let mut headers = [EMPTY_HEADER; 32];
    let mut req = Request::new(&mut headers);
    match req.parse(buf) {
        Ok(Status::Complete(_len)) => {
            do_handle_request(req, tcp_sender);
        }
        Ok(Status::Partial) => {
            warn!("unexpected httparse::Status::Partial");
        }
        Err(e) => {
            debug_warn!("error parsing request: {:?}", e);
        }
    }
}

#[derive(Debug)]
pub enum Conn {
    ReceivingRequestHeaders { buf: Vec<u8> },
    Finished,
}

impl Conn {
    pub fn new() -> Self {
        Conn::ReceivingRequestHeaders { buf: Vec::new() }
    }

    pub fn tcp_receive(&mut self, data: &[u8], tcp_sender: &ChannelSender) {
        match self {
            Conn::ReceivingRequestHeaders { buf } => {
                let mut handled = false;
                for line in data.split_inclusive(|&byte| byte == b'\n') {
                    if buf.len() + line.len() > REQUEST_MAX_SIZE {
                        warn!("request too large");
                        *self = Conn::Finished;
                        return;
                    }

                    buf.extend_from_slice(line);

                    if buf.ends_with(b"\r\n\r\n") {
                        handle_request(buf, tcp_sender);
                        handled = true;
                        break;
                    }

                    debug!("buf: {:?}", core::str::from_utf8(buf).unwrap());
                }

                if handled {
                    *self = Conn::Finished;
                }
            }
            Conn::Finished => {
                // Discard the received data.
            }
        }
    }
}
