use starina::prelude::*;

use crate::http::Body;
use crate::http::HeaderName;
use crate::http::Request;
use crate::http::RequestParser;
use crate::http::ResponseWriter;
use crate::http::StatusCode;

pub mod big;
pub mod index;

fn route(req: &Request, resp: &mut impl ResponseWriter) -> anyhow::Result<()> {
    match (&req.method, req.path.as_str()) {
        (crate::http::Method::Get, "/") => index::handle_index(req, resp),
        (crate::http::Method::Get, "/big") => big::handle_big(req, resp),
        _ => {
            error(resp, StatusCode::new(404).unwrap(), "Route not found");
            Ok(())
        }
    }
}

pub fn handle_http_request(
    parser: &mut RequestParser,
    resp: &mut impl ResponseWriter,
    data: &[u8],
) {
    match parser.parse_chunk(data) {
        Ok(Some(request)) => {
            info!(
                "HTTP Request - Method: {}, Path: {}",
                request.method, request.path
            );

            // Log headers
            for (name, value) in request.headers.iter() {
                info!("Header: {}: {}", name, value);
            }

            // Log body length (don't log the actual body content for security)
            match &request.body {
                Body::Full(body) => {
                    info!("Body length: {} bytes", body.len());
                }
            }

            if let Err(e) = route(&request, resp) {
                warn!("handler error: {:?}", e);
                error(resp, StatusCode::new(500).unwrap(), "Internal Server Error");
            }
        }
        Ok(None) => {
            // Need more data to complete the request
            trace!("Partial HTTP request received, waiting for more data");
        }
        Err(e) => {
            warn!("HTTP parsing error: {:?}", e);
            error(resp, StatusCode::new(400).unwrap(), "Bad Request");
        }
    }
}

fn error(resp: &mut impl ResponseWriter, status: StatusCode, message: &str) {
    if resp.sent_headers() {
        return;
    }

    let headers = resp.headers_mut();
    headers
        .insert(HeaderName::CONTENT_TYPE, "text/plain")
        .unwrap();

    resp.write_status(status);
    resp.write_body(message.as_bytes());

}
