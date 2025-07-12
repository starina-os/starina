use crate::http::Request;
use crate::http::ResponseWriter;
use crate::http::StatusCode;

pub fn handle_big(_req: &Request, resp: &mut impl ResponseWriter) -> anyhow::Result<()> {
    resp.write_status(StatusCode::OK);

    resp.write_body("A".repeat(4 * 1024).as_bytes());
    resp.write_body("B".repeat(4 * 1024).as_bytes());
    resp.write_body("C".repeat(4 * 1024).as_bytes());
    resp.write_body("D".repeat(4 * 1024).as_bytes());
    resp.write_body("E".repeat(4 * 1024).as_bytes());
    resp.write_body("F".repeat(4 * 1024).as_bytes());
    resp.write_body("G".repeat(4 * 1024).as_bytes());
    resp.write_body("H".repeat(4 * 1024).as_bytes());

    Ok(())
}
