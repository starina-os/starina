use core::mem;

use starina::prelude::*;
use thiserror::Error;

use super::Headers;
use super::Method;
use super::Request;
use super::request::Body;
use super::request::Query;

/// Per-connection state machine.
#[derive(Debug)]
enum State {
    ReadingStartLine,
    ReadingHeaders {
        method: Method,
        path: String,
        query: Query,
        headers: Headers,
    },
    ReadingBody {
        method: Method,
        path: String,
        query: Query,
        headers: Headers,
        content_length: usize,
    },
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Request too long")]
    TooLongRequest,
    #[error("Request contains non-UTF8 data")]
    NonUtf8Requst,
    #[error("Invalid start line")]
    InvalidStartLine,
    #[error("Unsupported HTTP version")]
    UnsupportedHttpVersion,
    #[error("Unsupported HTTP method")]
    UnsupportedMethod,
    #[error("Invalid header")]
    InvalidHeader,
    #[error("Empty header key")]
    EmptyHeaderKey,
    #[error("Unsupported transfer encoding")]
    UnsupportedTransferEncoding,
    #[error("Too many headers")]
    TooManyHeaders,
    #[error("Path too long")]
    PathTooLong,
    #[error("Header name too long")]
    HeaderNameTooLong,
    #[error("Header value too long")]
    HeaderValueTooLong,
    #[error("Invalid header name")]
    InvalidHeaderName,
}

const MAX_HEADERS_SIZE: usize = 16 * 1024;
const MAX_BODY_SIZE: usize = 1024 * 1024;
const MAX_PATH_LENGTH: usize = 8192;
const MAX_HEADER_NAME_LENGTH: usize = 256;
const MAX_HEADER_VALUE_LENGTH: usize = 8192;
const MAX_HEADER_COUNT: usize = 100;

pub struct RequestParser {
    headers_buf: String,
    body_buf: Vec<u8>,
    state: State,
}

impl RequestParser {
    pub fn new() -> Self {
        Self {
            state: State::ReadingStartLine,
            headers_buf: String::with_capacity(128),
            body_buf: Vec::new(),
        }
    }

    fn split_path_and_query(path_with_query: &str) -> (String, Query) {
        if let Some(pos) = path_with_query.find('?') {
            let path = path_with_query[..pos].to_string();
            let query_str = &path_with_query[pos + 1..];
            let query = Query::from_str(query_str);
            (path, query)
        } else {
            (path_with_query.to_string(), Query::new())
        }
    }

    fn reset(&mut self) {
        self.state = State::ReadingStartLine;
        self.headers_buf.clear();
        self.body_buf.clear();
    }

    pub fn parse_chunk(&mut self, chunk: &[u8]) -> Result<Option<Request>, Error> {
        if let State::ReadingBody { content_length, .. } = &self.state {
            let bytes_needed = content_length - self.body_buf.len();
            let bytes_to_take = chunk.len().min(bytes_needed);

            if self.body_buf.len() + bytes_to_take > MAX_BODY_SIZE {
                return Err(Error::TooLongRequest);
            }

            self.body_buf.extend_from_slice(&chunk[..bytes_to_take]);

            if self.body_buf.len() == *content_length {
                let State::ReadingBody {
                    method,
                    path,
                    query,
                    headers,
                    ..
                } = mem::replace(&mut self.state, State::ReadingStartLine)
                else {
                    unreachable!();
                };

                let request = Request {
                    method,
                    path,
                    query,
                    headers,
                    body: Body::Full(mem::take(&mut self.body_buf)),
                };

                // Reset parser state for next request.
                self.reset();

                return Ok(Some(request));
            }
            return Ok(None);
        }

        if self.headers_buf.len() + chunk.len() > MAX_HEADERS_SIZE {
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
                    let (Some(method), Some(path_and_query), Some(version)) =
                        (parts.next(), parts.next(), parts.next())
                    else {
                        return Err(Error::InvalidStartLine);
                    };

                    if version != "HTTP/1.1" && version != "HTTP/1.0" {
                        return Err(Error::UnsupportedHttpVersion);
                    }

                    if path_and_query.len() > MAX_PATH_LENGTH {
                        return Err(Error::PathTooLong);
                    }

                    let method_enum = Method::from_bytes(method.as_bytes())
                        .map_err(|_| Error::UnsupportedMethod)?;

                    let (path, query) = Self::split_path_and_query(path_and_query);

                    self.state = State::ReadingHeaders {
                        method: method_enum,
                        path,
                        query,
                        headers: Headers::new(),
                    };
                }
                State::ReadingHeaders { .. } if line == "\r\n" => {
                    // End of headers.
                    self.headers_buf = String::new();
                    let State::ReadingHeaders {
                        method,
                        path,
                        query,
                        headers,
                        ..
                    } = mem::replace(&mut self.state, State::ReadingStartLine)
                    else {
                        unreachable!();
                    };

                    // Check for unsupported transfer encodings.
                    if let Ok(Some(transfer_encoding)) = headers.get("transfer-encoding") {
                        if transfer_encoding.to_lowercase().contains("chunked") {
                            return Err(Error::UnsupportedTransferEncoding);
                        }
                    }

                    let content_length = headers
                        .get("content-length")
                        .ok()
                        .flatten()
                        .and_then(|v| v.parse::<usize>().ok())
                        .unwrap_or(0);

                    if content_length > MAX_BODY_SIZE {
                        return Err(Error::TooLongRequest);
                    }

                    if content_length == 0 {
                        let request = Request {
                            method,
                            path,
                            query,
                            headers,
                            body: Body::Full(Vec::new()),
                        };

                        // Reset parser state for next request.
                        self.reset();

                        return Ok(Some(request));
                    }

                    let body_chunk = &chunk[consumed_len..];
                    self.body_buf.extend_from_slice(body_chunk);

                    if self.body_buf.len() >= content_length {
                        let request = Request {
                            method,
                            path,
                            query,
                            headers,
                            body: Body::Full(self.body_buf[..content_length].to_vec()),
                        };

                        // Reset parser state for next request.
                        self.reset();

                        return Ok(Some(request));
                    }

                    self.state = State::ReadingBody {
                        method,
                        path,
                        query,
                        headers,
                        content_length,
                    };
                    return Ok(None);
                }
                State::ReadingHeaders { headers, .. } => {
                    if headers.len() >= MAX_HEADER_COUNT {
                        return Err(Error::TooManyHeaders);
                    }

                    let mut parts = line.trim_ascii_end().splitn(2, ':');
                    let (Some(key), Some(value)) = (parts.next(), parts.next()) else {
                        return Err(Error::InvalidHeader);
                    };

                    let key = key.trim();
                    let value = value.trim();

                    if key.is_empty() {
                        return Err(Error::EmptyHeaderKey);
                    }

                    if key.len() > MAX_HEADER_NAME_LENGTH {
                        return Err(Error::HeaderNameTooLong);
                    }

                    if value.len() > MAX_HEADER_VALUE_LENGTH {
                        return Err(Error::HeaderValueTooLong);
                    }

                    if !key.chars().all(|c| c.is_ascii_graphic() && c != ':') {
                        return Err(Error::InvalidHeaderName);
                    }

                    let _ = headers.append(key, value);
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

    use super::*;

    #[test]
    fn parse_simple_http_request() {
        let mut parser = RequestParser::new();
        let Ok(Some(request)) = parser.parse_chunk(b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n")
        else {
            panic!();
        };

        assert_eq!(request.method, Method::Get);
        assert_eq!(request.path, "/");
        assert_eq!(request.query.get("test"), None);
        assert_eq!(request.headers.get("host").unwrap().unwrap(), "example.com");
        assert!(matches!(request.body, Body::Full(ref body) if body.is_empty()));
    }

    #[test]
    fn parse_http_request_with_body() {
        let mut parser = RequestParser::new();
        let Ok(Some(request)) = parser.parse_chunk(
            b"POST /submit HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\n\r\nHello",
        ) else {
            panic!();
        };

        assert_eq!(request.method, Method::Post);
        assert_eq!(request.path, "/submit");
        assert_eq!(request.query.get("test"), None);
        assert_eq!(request.headers.get("content-length").unwrap().unwrap(), "5");
        assert!(matches!(request.body, Body::Full(ref body) if body == b"Hello"));
    }

    #[test]
    fn parse_http_request_with_partial_body() {
        let mut parser = RequestParser::new();
        assert!(matches!(
            parser.parse_chunk(
                b"POST /submit HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\n"
            ),
            Ok(None)
        ));

        let Ok(Some(request)) = parser.parse_chunk(b"\r\nHello") else {
            panic!();
        };

        assert_eq!(request.method, Method::Post);
        assert_eq!(request.path, "/submit");
        assert_eq!(request.query.get("test"), None);
        assert_eq!(request.headers.get("content-length").unwrap().unwrap(), "5");
        assert!(matches!(request.body, Body::Full(ref body) if body == b"Hello"));
    }

    #[test]
    fn parse_partial_http_request() {
        let mut parser = RequestParser::new();

        assert!(matches!(parser.parse_chunk(b"GE"), Ok(None)));
        assert!(matches!(parser.parse_chunk(b"T /path"), Ok(None)));
        assert!(matches!(
            parser.parse_chunk(b"/to HTTP/1.1\r\nHost"),
            Ok(None)
        ));
        assert!(matches!(parser.parse_chunk(b": example"), Ok(None)));
        assert!(matches!(parser.parse_chunk(b".com\r\n"), Ok(None)));

        let Ok(Some(request)) = parser.parse_chunk(b"\r\n") else {
            panic!();
        };

        assert_eq!(request.method, Method::Get);
        assert_eq!(request.path, "/path/to");
        assert_eq!(request.query.get("test"), None);
        assert_eq!(request.headers.get("host").unwrap().unwrap(), "example.com");
        assert!(matches!(request.body, Body::Full(ref body) if body.is_empty()));

        // Parser should be reset and ready for next request
        let Ok(Some(request2)) =
            parser.parse_chunk(b"GET /next HTTP/1.1\r\nHost: test.com\r\n\r\n")
        else {
            panic!();
        };
        assert_eq!(request2.method, Method::Get);
        assert_eq!(request2.path, "/next");
        assert_eq!(request2.query.get("test"), None);
    }

    #[test]
    fn parse_pipelined_requests() {
        let mut parser = RequestParser::new();

        // First request with body + start of second request in same chunk
        let chunk = b"POST /submit HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\n\r\nHelloGET / HTTP/1.1\r\nHost: test.com\r\n\r\n";

        // Should get the first request
        let Ok(Some(request1)) = parser.parse_chunk(chunk) else {
            panic!("Expected first request");
        };

        assert_eq!(request1.method, Method::Post);
        assert_eq!(request1.path, "/submit");
        assert_eq!(request1.query.get("test"), None);
        assert!(matches!(request1.body, Body::Full(ref body) if body == b"Hello"));

        // Parser should be reset and ready for the second request
        let Ok(Some(request2)) =
            parser.parse_chunk(b"GET /next HTTP/1.1\r\nHost: test.com\r\n\r\n")
        else {
            panic!("Expected to be able to parse second request");
        };

        assert_eq!(request2.method, Method::Get);
        assert_eq!(request2.path, "/next");
        assert_eq!(request2.query.get("test"), None);
    }

    #[test]
    fn parse_request_with_extra_bytes() {
        let mut parser = RequestParser::new();

        // Request with exact body length that ends mid-word
        let chunk = b"POST /api HTTP/1.1\r\nContent-Length: 3\r\n\r\nfoobar";

        let Ok(Some(request)) = parser.parse_chunk(chunk) else {
            panic!("Expected complete request");
        };

        assert_eq!(request.method, Method::Post);
        assert_eq!(request.path, "/api");
        assert_eq!(request.query.get("test"), None);
        assert!(matches!(request.body, Body::Full(ref body) if body == b"foo"));

        // Parser should be reset and ready for next request
        // Since we consumed exactly 3 bytes for body, "bar" should remain
        let Ok(Some(request2)) = parser.parse_chunk(b"GET /status HTTP/1.1\r\n\r\n") else {
            panic!("Expected second request to parse successfully");
        };

        assert_eq!(request2.method, Method::Get);
        assert_eq!(request2.path, "/status");
        assert_eq!(request2.query.get("test"), None);
    }

    #[test]
    fn parse_request_with_query_string() {
        let mut parser = RequestParser::new();
        let Ok(Some(request)) =
            parser.parse_chunk(b"GET /logs?off=100&limit=50 HTTP/1.1\r\nHost: example.com\r\n\r\n")
        else {
            panic!();
        };

        assert_eq!(request.method, Method::Get);
        assert_eq!(request.path, "/logs");
        assert_eq!(request.query.get("off"), Some("100"));
        assert_eq!(request.query.get("limit"), Some("50"));
        assert_eq!(request.query.get("nonexistent"), None);
        assert_eq!(request.headers.get("host").unwrap().unwrap(), "example.com");
        assert!(matches!(request.body, Body::Full(ref body) if body.is_empty()));
    }

    #[test]
    fn reject_chunked_transfer_encoding() {
        let mut parser = RequestParser::new();

        let chunk =
            b"POST /upload HTTP/1.1\r\nHost: example.com\r\nTransfer-Encoding: chunked\r\n\r\n";

        let result = parser.parse_chunk(chunk);
        assert!(matches!(result, Err(Error::UnsupportedTransferEncoding)));
    }

    #[test]
    fn reject_mixed_transfer_encoding_with_chunked() {
        let mut parser = RequestParser::new();

        let chunk = b"POST /upload HTTP/1.1\r\nHost: example.com\r\nTransfer-Encoding: gzip, chunked\r\n\r\n";

        let result = parser.parse_chunk(chunk);
        assert!(matches!(result, Err(Error::UnsupportedTransferEncoding)));
    }

    #[test]
    fn allow_content_length_requests() {
        let mut parser = RequestParser::new();

        let chunk = b"POST /upload HTTP/1.1\r\nHost: example.com\r\nContent-Length: 0\r\n\r\n";

        let result = parser.parse_chunk(chunk);
        assert!(matches!(result, Ok(Some(_))));
    }

    #[test]
    fn reject_too_long_path() {
        let mut parser = RequestParser::new();
        let long_path = "a".repeat(MAX_PATH_LENGTH + 1);
        let request = format!("GET /{} HTTP/1.1\r\n\r\n", long_path);

        let result = parser.parse_chunk(request.as_bytes());
        assert!(matches!(result, Err(Error::PathTooLong)));
    }

    #[test]
    fn reject_too_many_headers() {
        let mut parser = RequestParser::new();
        let mut request = "GET / HTTP/1.1\r\n".to_string();

        for i in 0..(MAX_HEADER_COUNT + 1) {
            request.push_str(&format!("Header{}: value\r\n", i));
        }
        request.push_str("\r\n");

        let result = parser.parse_chunk(request.as_bytes());
        assert!(matches!(result, Err(Error::TooManyHeaders)));
    }

    #[test]
    fn reject_too_long_header_name() {
        let mut parser = RequestParser::new();
        let long_name = "a".repeat(MAX_HEADER_NAME_LENGTH + 1);
        let request = format!("GET / HTTP/1.1\r\n{}: value\r\n\r\n", long_name);

        let result = parser.parse_chunk(request.as_bytes());
        assert!(matches!(result, Err(Error::HeaderNameTooLong)));
    }

    #[test]
    fn reject_too_long_header_value() {
        let mut parser = RequestParser::new();
        let long_value = "a".repeat(MAX_HEADER_VALUE_LENGTH + 1);
        let request = format!("GET / HTTP/1.1\r\nTest: {}\r\n\r\n", long_value);

        let result = parser.parse_chunk(request.as_bytes());
        assert!(matches!(result, Err(Error::HeaderValueTooLong)));
    }

    #[test]
    fn reject_invalid_header_name() {
        let mut parser = RequestParser::new();
        let request = b"GET / HTTP/1.1\r\nTest Header: value\r\n\r\n";

        let result = parser.parse_chunk(request);
        assert!(matches!(result, Err(Error::InvalidHeaderName)));
    }

    #[test]
    fn reject_header_name_with_control_char() {
        let mut parser = RequestParser::new();
        let request = b"GET / HTTP/1.1\r\nTest\x01Header: value\r\n\r\n";

        let result = parser.parse_chunk(request);
        assert!(matches!(result, Err(Error::InvalidHeaderName)));
    }
}
