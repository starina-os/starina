use crate::http::HeaderName;
use crate::http::Request;
use crate::http::ResponseWriter;
use crate::http::StatusCode;

static INDEX_HTML: &[u8] = include_bytes!("../../shell/index.html");

pub fn handle_index(_req: &Request, resp: &mut impl ResponseWriter) -> anyhow::Result<()> {
    let headers = resp.headers_mut();
    headers
        .insert(HeaderName::CONTENT_TYPE, "text/html")
        .unwrap();

    resp.write_headers(StatusCode::OK);
    resp.write_body(INDEX_HTML);

    Ok(())
}
