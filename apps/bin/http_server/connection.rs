use starina::channel::ChannelSender;
use starina::collections::HashMap;
use starina::error::ErrorCode;
use starina::message::StreamDataMsg;
use starina::prelude::*;

#[derive(Debug)]
pub struct StartLine {
    method: String,
    path: String,
}

/// Per-connection state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    ReadingStartLine,
    ReadingHeaders,
    ReadingBody,
    Errored,
}

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
    headers_buf: String,
    start_line: Option<StartLine>,
    headers: HashMap<String, Vec<String>>,
    remaining_headers_size: usize,
    state: State,
    tcp_writer: W,
}

impl<W: TcpWriter> Conn<W> {
    pub fn new(tcp_writer: W) -> Self {
        Self {
            state: State::ReadingStartLine,
            headers_buf: String::with_capacity(128),
            start_line: None,
            headers: HashMap::new(),
            remaining_headers_size: 16 * 1024,
            tcp_writer,
        }
    }

    fn on_headers(&mut self) {}

    fn on_body_chunk(&mut self, _chunk: &[u8]) {
        // Do something with the body chunk.
    }

    pub fn on_tcp_data(&mut self, chunk: &[u8]) {
        match self.state {
            State::ReadingBody => {
                self.on_body_chunk(chunk);
            }
            State::Errored => {
                // Do nothing. Ignore the data.
                return;
            }
            _ => {
                // Keep processing the data below.
            }
        }

        if self
            .remaining_headers_size
            .checked_sub(chunk.len())
            .is_none()
        {
            debug_warn!("too long HTTP request: state={:?}", self.state);
            self.state = State::Errored;
            return;
        }

        let Ok(chunk_str) = str::from_utf8(chunk) else {
            debug_warn!("non-utf-8 string in request start-line or headers");
            self.state = State::Errored;
            return;
        };

        self.headers_buf.push_str(chunk_str);
        let headers_buf = core::mem::take(&mut self.headers_buf);

        let mut consumed_len = 0;
        for line in headers_buf.split_inclusive("\r\n") {
            if !line.ends_with("\r\n") {
                // The line is still not terminated.
                break;
            }

            consumed_len += line.len();

            match self.state {
                State::ReadingStartLine => {
                    let mut parts = line.trim_ascii_end().splitn(3, ' ');
                    let (Some(method), Some(path), Some(version)) =
                        (parts.next(), parts.next(), parts.next())
                    else {
                        debug_warn!("invalid start-line: {}", line);
                        self.state = State::Errored;
                        return;
                    };

                    if version != "HTTP/1.1" && version != "HTTP/1.0" {
                        debug_warn!("unsupported HTTP version: {}", version);
                        self.state = State::Errored;
                        return;
                    }

                    let method_upper = method.to_uppercase();
                    match method_upper.as_str() {
                        "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS" => {}
                        _ => {
                            debug_warn!("unsupported HTTP method: {}", method);
                            self.state = State::Errored;
                            return;
                        }
                    }

                    self.start_line = Some(StartLine {
                        method: method_upper,
                        path: path.to_string(),
                    });
                    self.state = State::ReadingHeaders;
                }
                State::ReadingHeaders => {
                    if line == "\r\n" {
                        // End of headers.
                        self.state = State::ReadingBody;
                        self.on_body_chunk(headers_buf[consumed_len..].as_bytes());
                        consumed_len = headers_buf.len();
                        break;
                    }

                    let mut parts = line.trim_ascii_end().splitn(2, ':');
                    let (Some(key), Some(value)) = (parts.next(), parts.next()) else {
                        debug_warn!("invalid header: {}", line);
                        self.state = State::Errored;
                        return;
                    };

                    let key = key.trim().to_ascii_lowercase();
                    let value = value.trim().to_string();
                    if key.is_empty() {
                        debug_warn!("header key must not be empty");
                        self.state = State::Errored;
                        return;
                    }

                    self.headers.entry(key).or_insert_with(Vec::new).push(value);
                }
                _ => unreachable!(),
            }
        }

        self.headers_buf = headers_buf[consumed_len..].to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct MockTcpWriter(Vec<u8>);
    impl MockTcpWriter {
        pub fn new() -> Self {
            Self(Vec::new())
        }

        pub fn written_data(&self) -> &[u8] {
            &self.0
        }
    }

    impl TcpWriter for MockTcpWriter {
        fn write(&mut self, data: &[u8]) -> Result<(), ErrorCode> {
            self.0.extend_from_slice(data);
            Ok(())
        }
    }

    #[test]
    fn parse_simple_http_request() {
        let mut conn = Conn::new(MockTcpWriter::new());
        conn.on_tcp_data(b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n");
        assert_eq!(conn.state, State::ReadingBody);
        assert_eq!(conn.start_line.as_ref().unwrap().method, "GET");
        assert_eq!(conn.start_line.as_ref().unwrap().path, "/");
        assert_eq!(conn.headers.len(), 1);
        assert_eq!(conn.headers["host"], vec!["example.com"]);
        assert_eq!(conn.headers_buf.len(), 0);
    }

    #[test]
    fn parse_http_request_with_body() {
        let mut conn = Conn::new(MockTcpWriter::new());
        conn.on_tcp_data(
            b"POST /submit HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\n\r\nHello",
        );
        assert_eq!(conn.state, State::ReadingBody);
        assert_eq!(conn.start_line.as_ref().unwrap().method, "POST");
        assert_eq!(conn.start_line.as_ref().unwrap().path, "/submit");
        assert_eq!(conn.headers.len(), 2);
        assert_eq!(conn.headers["content-length"], vec!["5"]);
        assert_eq!(conn.headers_buf.len(), 0);
    }

    #[test]
    fn parse_partial_http_request() {
        let mut conn = Conn::new(MockTcpWriter::new());

        conn.on_tcp_data(b"GE");
        assert_eq!(conn.state, State::ReadingStartLine);

        conn.on_tcp_data(b"T /path");
        assert_eq!(conn.state, State::ReadingStartLine);

        conn.on_tcp_data(b"/to HTTP/1.1\r\nHost");
        assert_eq!(conn.state, State::ReadingHeaders);

        conn.on_tcp_data(b": example");
        assert_eq!(conn.state, State::ReadingHeaders);

        conn.on_tcp_data(b".com\r\n");
        assert_eq!(conn.state, State::ReadingHeaders);

        conn.on_tcp_data(b"\r\n");
        assert_eq!(conn.state, State::ReadingBody);

        conn.on_tcp_data(b"Hello");
        assert_eq!(conn.state, State::ReadingBody);

        conn.on_tcp_data(b"World");
        assert_eq!(conn.state, State::ReadingBody);
    }
}
