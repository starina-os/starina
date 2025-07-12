use starina::prelude::*;

use crate::http::Request;
use crate::http::ResponseWriter;
use crate::http::StatusCode;

pub fn handle_index(req: &Request, resp: &mut impl ResponseWriter) -> anyhow::Result<()> {
    resp.write_status(StatusCode::OK);
    resp.write_body(
        format!(
            "Hello from Starina API! You requested: {} {}",
            req.method, req.path
        )
        .as_bytes(),
    );

    Ok(())
}
