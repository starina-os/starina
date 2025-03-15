use crate::header::Headers;
use crate::request::Request;
use crate::status::StatusCode;
pub trait ResponseWriter {
    fn write_headers(&mut self, status: StatusCode, headers: Headers);
    fn write_body(&mut self, data: &[u8]);
}

pub trait Handler {
    fn handle(&self, request: Request, writer: impl ResponseWriter);
}
