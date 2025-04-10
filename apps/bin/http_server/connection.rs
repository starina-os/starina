use starina::channel::ChannelSender;
use starina::error::ErrorCode;
use starina::message::StreamDataMsg;
use starina::prelude::*;

use crate::http::request_parser::HttpRequestParser;
use crate::http::request_parser::Part;
use crate::http::response_writer::HttpResponseWriter;
use crate::http::response_writer::Writer;

pub struct ChannelWriter(ChannelSender);

impl ChannelWriter {
    pub fn new(tcpip_sender: ChannelSender) -> Self {
        Self(tcpip_sender)
    }
}

impl Writer for ChannelWriter {
    type Error = ErrorCode;

    fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.0.send(StreamDataMsg { data: buf })
    }
}

pub struct Conn<W: Writer> {
    response_writer: Option<HttpResponseWriter<W>>,
    request_parser: HttpRequestParser,
}

impl<W: Writer> Conn<W> {
    pub fn new(writer: W) -> Self {
        Self {
            response_writer: Some(HttpResponseWriter::new(writer)),
            request_parser: HttpRequestParser::new(),
        }
    }

    pub fn on_tcp_data(&mut self, chunk: &[u8]) {
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
                        let mut response_writer = self.response_writer.take().unwrap();
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
                return;
            }
        }
    }
}
