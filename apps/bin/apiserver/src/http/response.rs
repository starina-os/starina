use starina::channel::Channel;
use starina::error::ErrorCode;
use starina::message::Message;
use starina::prelude::*;

use crate::http::Headers;
use crate::http::StatusCode;

pub trait ResponseWriter {
    fn write_status(&mut self, status: StatusCode);
    fn headers_mut(&mut self) -> &mut Headers;
    fn write_body(&mut self, data: &[u8]);
    fn finish(&mut self) -> Result<(), ErrorCode>;
    fn sent_headers(&self) -> bool;
}

pub struct BufferedResponseWriter<'a> {
    status: Option<StatusCode>,
    headers: Headers,
    body: Vec<u8>,
    channel: &'a Channel,
}

impl<'a> BufferedResponseWriter<'a> {
    pub fn new(channel: &'a Channel) -> Self {
        Self {
            status: None,
            headers: Headers::new(),
            body: Vec::new(),
            channel,
        }
    }
}

impl<'a> ResponseWriter for BufferedResponseWriter<'a> {
    fn write_status(&mut self, status: StatusCode) {
        self.status = Some(status);
    }

    fn headers_mut(&mut self) -> &mut Headers {
        &mut self.headers
    }

    fn write_body(&mut self, data: &[u8]) {
        self.body.extend_from_slice(data);
    }

    fn finish(&mut self) -> Result<(), ErrorCode> {
        let status = self.status.unwrap_or(StatusCode::OK);
        let mut response = format!("HTTP/1.1 {}\r\n", status.as_u16());

        response.push_str("Connection: close\r\n");

        for (name, value) in self.headers.iter() {
            response.push_str(&format!("{}: {}\r\n", name, value));
        }

        response.push_str(&format!("Content-Length: {}\r\n\r\n", self.body.len()));

        self.channel.send(Message::Data {
            data: response.as_bytes(),
        })?;

        if !self.body.is_empty() {
            self.channel.send(Message::Data { data: &self.body })?;
        }

        Ok(())
    }

    fn sent_headers(&self) -> bool {
        self.status.is_some()
    }
}
