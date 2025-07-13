use starina::channel::ChannelSender;
use starina::error::ErrorCode;
use starina::message::MESSAGE_DATA_LEN_MAX;
use starina::message::Message;
use starina::prelude::*;

use crate::http::Headers;
use crate::http::StatusCode;

#[derive(Debug)]
pub enum TryFlushResult {
    Done,
    Partial,
    Error(ErrorCode),
}

pub trait ResponseWriter {
    fn headers_mut(&mut self) -> &mut Headers;
    fn write_headers(&mut self, status: StatusCode);
    fn write_body(&mut self, data: &[u8]);
    fn are_headers_sent(&self) -> bool;
    fn try_flush(&mut self) -> TryFlushResult;
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

    fn send_chunk(&self, data: &[u8], byte_index: &mut usize) -> Result<bool, ErrorCode> {
        let remaining = &data[*byte_index..];
        let chunk_size = remaining.len().min(MESSAGE_DATA_LEN_MAX);
        let chunk = &remaining[..chunk_size];

        match self.channel.send(Message::Data { data: chunk }) {
            Ok(()) => {
                *byte_index += chunk_size;
                Ok(true)
            }
            Err(ErrorCode::Full) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

impl ResponseWriter for BufferedResponseWriter {
    fn write_headers(&mut self, status_code: StatusCode) {
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

    fn try_flush(&mut self) -> TryFlushResult {
        loop {
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
                    debug!("Building response headers with body length: {}", body.len());

                    self.state = ResponseState::SendingHeaders {
                        headers: response,
                        index: 0,
                        body,
                    };
                }
                ResponseState::SendingHeaders {
                    headers: flatten_headers,
                    mut index,
                    body,
                } => {
                    match self.send_chunk(flatten_headers.as_bytes(), &mut index) {
                        Ok(true) => {}
                        Ok(false) => {
                            self.state = ResponseState::SendingHeaders {
                                headers: flatten_headers,
                                index,
                                body,
                            };
                            return TryFlushResult::Partial;
                        }
                        Err(e) => return TryFlushResult::Error(e),
                    }
                    if index >= flatten_headers.len() {
                        self.state = ResponseState::SendingBody { body, index: 0 };
                        continue;
                    }

                    self.state = ResponseState::SendingHeaders {
                        headers: flatten_headers,
                        index,
                        body,
                    };
                }
                ResponseState::SendingBody { body, mut index } => {
                    debug!("SendingBody: index={}, body.len()={}", index, body.len());
                    match self.send_chunk(&body, &mut index) {
                        Ok(true) => {}
                        Ok(false) => {
                            debug!(
                                "send_chunk returned false (backpressure), index now={}",
                                index
                            );
                            self.state = ResponseState::SendingBody { body, index };
                            return TryFlushResult::Partial;
                        }
                        Err(e) => return TryFlushResult::Error(e),
                    }
                    debug!("send_chunk succeeded, index now={}", index);
                    if index >= body.len() {
                        debug!("Body fully sent, finishing");
                        self.state = ResponseState::Finished;
                        return TryFlushResult::Done;
                    }

                    debug!("More body data to send");
                    self.state = ResponseState::SendingBody { body, index };
                    return TryFlushResult::Partial;
                }
                ResponseState::Finished => {
                    self.state = ResponseState::Finished;
                    return TryFlushResult::Done;
                }
            }
        }
    }

    fn are_headers_sent(&self) -> bool {
        !matches!(self.state, ResponseState::BeforeHeaders { .. })
    }
}
