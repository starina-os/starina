use starina::channel::ChannelSender;
use starina::error::ErrorCode;
use starina::message::MESSAGE_DATA_LEN_MAX;
use starina::message::Message;
use starina::prelude::*;

use crate::http::Headers;
use crate::http::StatusCode;

pub trait ResponseWriter {
    fn write_status(&mut self, status: StatusCode);
    fn headers_mut(&mut self) -> &mut Headers;
    fn write_body(&mut self, data: &[u8]);
    fn flush(&mut self) -> Result<bool, ErrorCode>;
    fn sent_headers(&self) -> bool;
}

enum ResponseState {
    BeforeHeaders {
        status: Option<StatusCode>,
        headers: Headers,
        body: Vec<u8>,
    },
    SendingHeaders {
        headers: String,
        index: usize,
        body: Vec<u8>,
    },
    SendingBody {
        body: Vec<u8>,
        index: usize,
    },
    Finished,
}

pub struct BufferedResponseWriter {
    state: ResponseState,
    channel: ChannelSender,
}

impl BufferedResponseWriter {
    pub fn new(channel: ChannelSender) -> Self {
        Self {
            state: ResponseState::BeforeHeaders {
                status: None,
                headers: Headers::new(),
                body: Vec::new(),
            },
            channel,
        }
    }

    fn send_chunk(&self, data: &[u8], byte_index: &mut usize) -> Result<(), ErrorCode> {
        let remaining = &data[*byte_index..];
        let chunk_size = remaining.len().min(MESSAGE_DATA_LEN_MAX);
        let chunk = &remaining[..chunk_size];

        self.channel.send(Message::Data { data: chunk })?;
        *byte_index += chunk_size;
        Ok(())
    }
}

impl ResponseWriter for BufferedResponseWriter {
    fn write_status(&mut self, status_code: StatusCode) {
        match &mut self.state {
            ResponseState::BeforeHeaders { status, .. } => {
                *status = Some(status_code);
            }
            _ => panic!("Cannot write status after headers have been sent"),
        }
    }

    fn headers_mut(&mut self) -> &mut Headers {
        match &mut self.state {
            ResponseState::BeforeHeaders { headers, .. } => headers,
            _ => panic!("Cannot modify headers after they have been sent"),
        }
    }

    fn write_body(&mut self, data: &[u8]) {
        match &mut self.state {
            ResponseState::BeforeHeaders { body, .. }
            | ResponseState::SendingHeaders { body, .. }
            | ResponseState::SendingBody { body, .. } => {
                body.extend_from_slice(data);
            }
            _ => panic!("Cannot write body after flush() has been called"),
        }
    }

    fn flush(&mut self) -> Result<bool, ErrorCode> {
        match core::mem::replace(&mut self.state, ResponseState::Finished) {
            ResponseState::BeforeHeaders {
                status,
                headers,
                body,
            } => {
                let status_code = status.unwrap_or(StatusCode::OK);
                let mut response = format!("HTTP/1.1 {}\r\n", status_code.as_u16());
                response.push_str("Connection: close\r\n");

                for (name, value) in headers.iter() {
                    response.push_str(&format!("{}: {}\r\n", name, value));
                }

                response.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));

                self.state = ResponseState::SendingHeaders {
                    headers: response,
                    index: 0,
                    body,
                };
                Ok(false)
            }
            ResponseState::SendingHeaders {
                headers: flatten_headers,
                mut index,
                body,
            } => {
                self.send_chunk(flatten_headers.as_bytes(), &mut index)?;
                if index >= flatten_headers.len() {
                    self.state = ResponseState::SendingBody { body, index: 0 };
                    return Ok(false);
                }

                self.state = ResponseState::SendingHeaders {
                    headers: flatten_headers,
                    index,
                    body,
                };
                Ok(false)
            }
            ResponseState::SendingBody { body, mut index } => {
                self.send_chunk(&body, &mut index)?;
                if index >= body.len() {
                    self.state = ResponseState::Finished;
                    return Ok(true);
                }

                self.state = ResponseState::SendingBody { body, index };
                Ok(false)
            }
            ResponseState::Finished => {
                self.state = ResponseState::Finished;
                Ok(true)
            }
        }
    }

    fn sent_headers(&self) -> bool {
        !matches!(self.state, ResponseState::BeforeHeaders { .. })
    }
}
