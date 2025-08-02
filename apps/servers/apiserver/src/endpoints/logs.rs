use starina::syscall;

use crate::http::HeaderName;
use crate::http::Request;
use crate::http::ResponseWriter;
use crate::http::StatusCode;

pub fn handle_logs(_req: &Request, resp: &mut impl ResponseWriter) -> anyhow::Result<()> {
    let mut buffer = [0; 4096];
    let read_len = match syscall::log_read(&mut buffer) {
        Ok(len) => len,
        Err(_) => {
            resp.write_headers(StatusCode::new(500).unwrap());
            resp.write_body(b"failed to read logs");
            return Ok(());
        }
    };

    let headers = resp.headers_mut();
    headers.insert(HeaderName::CONTENT_TYPE, "text/plain")?;

    resp.write_headers(StatusCode::new(200).unwrap());
    resp.write_body(&buffer[..read_len]);

    Ok(())
}
