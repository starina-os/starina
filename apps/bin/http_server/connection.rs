use starina::channel::ChannelSender;
use starina::error::ErrorCode;
use starina::message::StreamDataMsg;
use starina::prelude::*;

use crate::http::HttpRequestParser;
use crate::http::Part;

pub trait TcpWriter {
    fn write(&mut self, data: &[u8]) -> Result<(), ErrorCode>;
}

pub struct ChannelTcpWriter(ChannelSender);

impl ChannelTcpWriter {
    pub fn new(tcpip_sender: ChannelSender) -> Self {
        Self(tcpip_sender)
    }
}

impl TcpWriter for ChannelTcpWriter {
    fn write(&mut self, data: &[u8]) -> Result<(), ErrorCode> {
        self.0.send(StreamDataMsg { data })
    }
}

pub struct Conn<W: TcpWriter> {
    tcp_writer: W,
    request_parser: HttpRequestParser,
}

impl<W: TcpWriter> Conn<W> {
    pub fn new(tcp_writer: W) -> Self {
        Self {
            tcp_writer,
            request_parser: HttpRequestParser::new(),
        }
    }

    pub fn on_tcp_data(&mut self, chunk: &[u8]) {
        loop {
            match self.request_parser.parse_chunk(chunk) {
                Ok(Some(part)) => {
                    warn!("{:?}", part);
                    match part {
                        Part::Request {
                            method,
                            path,
                            headers,
                            first_body,
                        } => {
                            // Do something.
                        }
                        Part::Body { chunk } => {
                            // Do something.
                        }
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    warn!("HTTP parse error: {:?}", err);
                    // TODO: close the connection
                    return;
                }
            }
        }
    }
}
