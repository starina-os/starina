use httparse::Request;
use httparse::Status;
use httparse::EMPTY_HEADER;
use starina_api::channel::ChannelSender;
use starina_api::prelude::*;

use crate::starina_autogen::idl::tcpip::TcpSend;

const REQUEST_MAX_SIZE: usize = 32 * 1024;

const INDEX_HTML: &[u8] = include_bytes!("index.html");
const NOT_FOUND_HTML: &[u8] = include_bytes!("404.html");

fn do_handle_request(req: Request, tcp_sender: &ChannelSender) {
    info!("{} {}", req.method.unwrap(), req.path.unwrap());

    let (status, body) = match req.path {
        Some("/index.html" | "/") => ("200 OK", INDEX_HTML),
        _ => ("404 Not Found", NOT_FOUND_HTML),
    };

    let headers = format!(
        "HTTP/1.1 {}\r\nContent-Type: text/html\r\nX-Powered-by: Starina\r\nContent-Length: {}\r\n\r\n",
        status,
        body.len()
    );

    tcp_sender
        .send(TcpSend {
            data: headers.as_bytes().try_into().unwrap(),
        })
        .unwrap();

    for chunk in body.chunks(2048) {
        tcp_sender
            .send(TcpSend {
                data: chunk.try_into().unwrap(),
            })
            .unwrap();
    }
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
