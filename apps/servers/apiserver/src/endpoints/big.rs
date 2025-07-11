use crate::http::Request;
use crate::http::ResponseWriter;
use crate::http::StatusCode;

pub fn handle_big(_req: &Request, resp: &mut impl ResponseWriter) -> anyhow::Result<()> {
    resp.write_status(StatusCode::OK);
    
    // Generate 16KB of text
    let text = "A".repeat(16 * 1024);
    resp.write_body(text.as_bytes());

    loop {
        let finished = resp.flush()
            .map_err(|e| anyhow::anyhow!("Failed to flush response: {:?}", e))?;
        if finished {
            break;
        }
    }
    Ok(())
}