use starina::channel::ChannelSender;
use starina::error::ErrorCode;
use starina::message::StreamDataMsg;
use starina::prelude::*;

/// Per-connection state machine.
#[derive(Debug)]
enum State {
    ReadingRequestLine { buf: String },
    ReadingHeaders {},
}

pub struct TcpWriter(ChannelSender);

impl TcpWriter {
    pub fn new(tcpip_sender: ChannelSender) -> Self {
        Self(tcpip_sender)
    }

    pub fn send(&self, data: &[u8]) -> Result<(), ErrorCode> {
        self.0.send(StreamDataMsg { data })
    }
}

pub struct Conn {
    state: State,
    tcp_writer: TcpWriter,
}

impl Conn {
    pub fn new(tcp_writer: TcpWriter) -> Self {
        Self {
            state: State::ReadingRequestLine { buf: String::new() },
            tcp_writer,
        }
    }

    pub fn on_data(&mut self, chunk: &[u8]) {
        // TODO:
    }
}
