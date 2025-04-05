use starina::collections::HashMap;
use starina::prelude::*;

/// Per-connection state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    ReadingStartLine,
    ReadingHeaders {
        method: String,
        path: String,
        headers: HashMap<String, Vec<String>>,
    },
    ReadingBody,
    Errored,
}

#[derive(Debug)]
pub enum Part<'a> {
    Request {
        method: String,
        path: String,
        headers: HashMap<String, Vec<String>>,
        first_body: &'a [u8],
    },
    Body {
        chunk: &'a [u8],
    },
}

#[derive(Debug)]
pub enum Error {
    Errored,
    TooLongRequest,
    NonUtf8Requst,
    InvalidStartLine,
    UnsupportedHttpVersion,
    UnsupportedMethod,
    InvalidHeader,
    EmptyHeaderKey,
}

pub struct HttpRequestParser {
    headers_buf: String,
    remaining_headers_size: usize,
    state: State,
}

impl HttpRequestParser {
    pub fn new() -> Self {
        Self {
            state: State::ReadingStartLine,
            headers_buf: String::with_capacity(128),
            remaining_headers_size: 16 * 1024,
        }
    }

    pub fn parse_chunk<'a>(&mut self, chunk: &'a [u8]) -> Result<Option<Part<'a>>, Error> {
        let result = self.do_parse_chunk(chunk);
        if result.is_err() {
            self.state = State::Errored;
        }

        result
    }

    fn do_parse_chunk<'a>(&mut self, chunk: &'a [u8]) -> Result<Option<Part<'a>>, Error> {
        match self.state {
            State::ReadingBody => {
                return Ok(Some(Part::Body { chunk }));
            }
            State::Errored => {
                // Do nothing. Ignore the data.
                return Err(Error::Errored);
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
            return Err(Error::TooLongRequest);
        }

        let Ok(chunk_str) = str::from_utf8(chunk) else {
            return Err(Error::NonUtf8Requst);
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

            match &mut self.state {
                State::ReadingStartLine => {
                    let mut parts = line.trim_ascii_end().splitn(3, ' ');
                    let (Some(method), Some(path), Some(version)) =
                        (parts.next(), parts.next(), parts.next())
                    else {
                        return Err(Error::InvalidStartLine);
                    };

                    if version != "HTTP/1.1" && version != "HTTP/1.0" {
                        return Err(Error::UnsupportedHttpVersion);
                    }

                    let method_upper = method.to_uppercase();
                    match method_upper.as_str() {
                        "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS" => {}
                        _ => {
                            return Err(Error::UnsupportedMethod);
                        }
                    }

                    self.state = State::ReadingHeaders {
                        method: method_upper,
                        path: path.to_string(),
                        headers: HashMap::new(),
                    };
                }
                State::ReadingHeaders { .. } if line == "\r\n" => {
                    // End of headers.
                    let body_chunk = &chunk[(self.headers_buf.len() - consumed_len)..];
                    self.headers_buf = String::new();
                    let State::ReadingHeaders {
                        method,
                        path,
                        headers,
                        ..
                    } = core::mem::replace(&mut self.state, State::ReadingBody)
                    else {
                        unreachable!();
                    };

                    let part = Part::Request {
                        method,
                        path,
                        headers,
                        first_body: body_chunk,
                    };

                    return Ok(Some(part));
                }
                State::ReadingHeaders { headers, .. } => {
                    let mut parts = line.trim_ascii_end().splitn(2, ':');
                    let (Some(key), Some(value)) = (parts.next(), parts.next()) else {
                        return Err(Error::InvalidHeader);
                    };

                    let key = key.trim().to_ascii_lowercase();
                    let value = value.trim().to_string();
                    if key.is_empty() {
                        return Err(Error::EmptyHeaderKey);
                    }

                    headers.entry(key).or_insert_with(Vec::new).push(value);
                }
                _ => unreachable!(),
            }
        }

        self.headers_buf = headers_buf[consumed_len..].to_owned();
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use starina::error::ErrorCode;

    use super::*;

    #[test]
    fn parse_simple_http_request() {
        let mut parser = HttpRequestParser::new();
        let Ok(Some(Part::Request {
            method,
            path,
            headers,
            first_body,
        })) = parser.parse_chunk(b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n")
        else {
            panic!();
        };

        assert_eq!(method, "GET");
        assert_eq!(path, "/");
        assert_eq!(headers.len(), 1);
        assert_eq!(headers["host"], vec!["example.com"]);
        assert_eq!(first_body.len(), 0);
    }

    #[test]
    fn parse_http_request_with_body() {
        let mut parser = HttpRequestParser::new();
        let Ok(Some(Part::Request {
            method,
            path,
            headers,
            first_body,
        })) = parser.parse_chunk(
            b"POST /submit HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\n\r\nHello",
        )
        else {
            panic!();
        };

        assert_eq!(method, "POST");
        assert_eq!(path, "/submit");
        assert_eq!(headers.len(), 2);
        assert_eq!(headers["content-length"], vec!["5"]);
        assert_eq!(first_body, b"Hello");
    }
    #[test]
    fn parse_partial_http_request() {
        let mut parser = HttpRequestParser::new();

        assert!(matches!(parser.parse_chunk(b"GE"), Ok(None)));
        assert!(matches!(parser.parse_chunk(b"T /path"), Ok(None)));
        assert!(matches!(
            parser.parse_chunk(b"/to HTTP/1.1\r\nHost"),
            Ok(None)
        ));
        assert!(matches!(parser.parse_chunk(b": example"), Ok(None)));
        assert!(matches!(parser.parse_chunk(b".com\r\n"), Ok(None)));

        let Ok(Some(Part::Request {
            method,
            path,
            headers,
            first_body,
        })) = parser.parse_chunk(b"\r\n")
        else {
            panic!();
        };

        assert_eq!(method, "GET");
        assert_eq!(path, "/path/to");
        assert_eq!(headers.len(), 1);
        assert_eq!(headers["host"], vec!["example.com"]);
        assert_eq!(first_body.len(), 0);

        assert!(matches!(
            parser.parse_chunk(b"Hello"),
            Ok(Some(Part::Body { chunk }))
        ));
        assert!(matches!(
            parser.parse_chunk(b"World"),
            Ok(Some(Part::Body { chunk }))
        ));
    }
}
