use core::fmt;

use starina::channel::ChannelSender;
use starina::error::ErrorCode;
use starina::message::StreamDataMsg;
use starina::prelude::*;

use crate::http::request_parser::HttpRequestParser;
use crate::http::request_parser::Part;
use crate::http::response_writer::HttpResponseWriter;
use crate::http::response_writer::Writer;

pub struct ChannelWriter<'a>(&'a ChannelSender);

impl<'a> ChannelWriter<'a> {
    pub fn new(tcpip_sender: &'a ChannelSender) -> Self {
        Self(tcpip_sender)
    }
}

impl<'a> Writer for ChannelWriter<'a> {
    type Error = ErrorCode;

    fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.0.send(StreamDataMsg { data: buf })
    }
}

pub struct Conn {
    request_parser: HttpRequestParser,
}

impl Conn {
    pub fn new() -> Self {
        Self {
            request_parser: HttpRequestParser::new(),
        }
    }

    pub fn on_tcp_data<W: Writer>(&mut self, writer: W, chunk: &[u8]) {
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
                        // TODO: backpressure
                        let mut response_writer = HttpResponseWriter::new(writer);
                        response_writer.set_header("server", "Starina").unwrap();
                        response_writer.set_header("connection", "close").unwrap();
                        response_writer.write_status(200).unwrap();
                        response_writer
                            .write_body(format!("You sent: {} {}", method, path).as_bytes())
                            .unwrap();
                        drop(response_writer);
                    }
                    Part::Body { chunk } => {
                        // Do something.
                    }
                }
            }
            Ok(None) => {
                // Needs more data.
            }
            Err(err) => {
                warn!("HTTP parse error: {:?}", err);
                // TODO: close the connection
            }
        }
    }
}

impl fmt::Debug for Conn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Conn").finish()
    }
}
