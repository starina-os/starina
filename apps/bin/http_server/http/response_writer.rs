use core::fmt::Write;

use starina::prelude::*;

enum State {
    BeforeHeaders { headers_text: String },
    Body,
}

pub struct HttpResponseWriter<W: Write> {
    state: State,
    writer: W,
}

impl<W: Write> HttpResponseWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            state: State::BeforeHeaders,
            writer,
        }
    }

    pub fn set_header(&mut self, name: &str, value: &str) -> Result<(), W::Error> {
        let State::BeforeHeaders { headers_text } = &mut self.state else {
            panic!("cannot set header after body has started");
        };

        write!(headers_text, "{}: {}\r\n", name, value)
    }

    pub fn write_status(&mut self, status: u16) -> Result<(), W::Error> {
        let State::BeforeHeaders { headers_text } = &mut self.state else {
            panic!("cannot write status twice");
        };

        write!(&mut self.writer, "HTTP/1.1 {}\r\n", status)?;
        self.writer.write_str("connection: close\r\n")?;
        self.writer.write_str("transfer-encoding: chunked\r\n")?;
        self.writer.write_all(headers_text.as_bytes())?;
        self.writer.write_all(b"\r\n")?;
        self.state = State::Body;
        Ok(())
    }

    pub fn write_body(&mut self, chunk: &[u8]) -> Result<(), W::Error> {
        if matches!(self.state, State::BeforeHeaders) {
            self.write_status(200)?;
        }

        // Chunked transfer encoding.
        write!(&mut self.writer, "{:x}\r\n", chunk.len())?;
        self.writer.write_all(chunk)?;
        self.writer.write_all(b"\r\n")?;
        Ok(())
    }
}

impl<W: Write> Drop for HttpResponseWriter<W> {
    fn drop(&mut self) {
        match self.state {
            State::BeforeHeaders { headers_text } => {
                // If we drop the writer before writing the headers, handle it as an error.
                let _ = self.write_status(500);
            }
            State::Body => {
                // The end of response body. Chunked encoding.
                let _ = self.writer.write_all(b"0\r\n\r\n");
            }
        }
    }
}
