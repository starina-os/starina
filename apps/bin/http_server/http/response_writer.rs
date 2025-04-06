use core::fmt;

use starina::prelude::*;

pub trait Writer {
    type Error;

    fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error>;
}

struct WriterWrapper<W: Writer> {
    writer: W,
    error: Option<W::Error>,
}

impl<W: Writer> fmt::Write for WriterWrapper<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self.write(s.as_bytes()) {
            Ok(()) => Ok(()),
            Err(err) => {
                self.error = Some(err);
                Err(fmt::Error)
            }
        }
    }
}

impl<W: Writer> WriterWrapper<W> {
    fn write(&mut self, buf: &[u8]) -> Result<(), W::Error> {
        self.writer.write(buf)
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<(), W::Error> {
        match fmt::write(self, args) {
            Ok(()) => Ok(()),
            Err(_) => {
                let err = self.error.take().expect("fmt::write failed");
                Err(err)
            }
        }
    }
}

enum State {
    BeforeHeaders { headers_text: String },
    Body,
}

pub struct HttpResponseWriter<W: Writer> {
    state: State,
    writer: WriterWrapper<W>,
}

impl<W: Writer> HttpResponseWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            state: State::BeforeHeaders {
                headers_text: String::new(),
            },
            writer: WriterWrapper {
                writer,
                error: None,
            },
        }
    }

    pub fn set_header(&mut self, name: &str, value: &str) -> Result<(), fmt::Error> {
        use core::fmt::Write;

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
        self.writer.write(b"connection: close\r\n")?;
        self.writer.write(b"transfer-encoding: chunked\r\n")?;
        self.writer.write(headers_text.as_bytes())?;
        self.writer.write(b"\r\n")?;
        self.state = State::Body;
        Ok(())
    }

    pub fn write_body(&mut self, chunk: &[u8]) -> Result<(), W::Error> {
        if matches!(self.state, State::BeforeHeaders { .. }) {
            self.write_status(200)?;
        }

        // Chunked transfer encoding.
        write!(&mut self.writer, "{:x}\r\n", chunk.len())?;
        self.writer.write(chunk)?;
        self.writer.write(b"\r\n")?;
        Ok(())
    }
}

impl<W: Writer> Drop for HttpResponseWriter<W> {
    fn drop(&mut self) {
        match &self.state {
            State::BeforeHeaders { .. } => {
                // If we drop the writer before writing the headers, handle it as an error.
                let _ = self.write_status(500);
            }
            State::Body => {
                // The end of response body. Chunked encoding.
                let _ = self.writer.write(b"0\r\n\r\n");
            }
        }
    }
}
