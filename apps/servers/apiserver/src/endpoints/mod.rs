use starina::prelude::*;

use crate::http::HeaderName;
use crate::http::Method;
use crate::http::Request;
use crate::http::ResponseWriter;
use crate::http::StatusCode;

pub mod big;
pub mod index;
pub mod logs;

pub fn route(req: &Request, resp: &mut impl ResponseWriter) -> anyhow::Result<()> {
    match (&req.method, req.path.as_str()) {
        (Method::Get, "/") => index::handle_index(req, resp),
        (Method::Get, "/big") => big::handle_big(req, resp),
        (Method::Get, "/logs") => logs::handle_logs(req, resp),
        _ => {
            error(resp, StatusCode::new(404).unwrap(), "Route not found");
            Ok(())
        }
    }
}

pub fn error(resp: &mut impl ResponseWriter, status: StatusCode, message: &str) {
    if resp.are_headers_sent() {
        // It's too late to send an error response.
        debug_warn!(
            "HTTP error response already sent, cannot send error: {}",
            message
        );
        return;
    }

    let headers = resp.headers_mut();
    headers
        .insert(HeaderName::CONTENT_TYPE, "text/plain")
        .unwrap();

    resp.write_headers(status);
    resp.write_body(message);
}
