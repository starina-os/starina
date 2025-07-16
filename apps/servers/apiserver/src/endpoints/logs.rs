use starina::prelude::*;
use starina::syscall;

use crate::http::HeaderName;
use crate::http::Request;
use crate::http::ResponseWriter;
use crate::http::StatusCode;

pub fn handle_logs(req: &Request, resp: &mut impl ResponseWriter) -> anyhow::Result<()> {
    let offset = req
        .query
        .get("offset")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let limit = req
        .query
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(4096)
        .min(16 * 1024);

    let mut buffer = vec![0u8; limit];
    let read_len = syscall::log_read(offset, &mut buffer).unwrap_or(0);
    buffer.truncate(read_len);

    let headers = resp.headers_mut();
    headers.insert(HeaderName::CONTENT_TYPE, "text/plain")?;

    resp.write_headers(StatusCode::new(200).unwrap());
    resp.write_body(buffer.as_slice());

    Ok(())
}
