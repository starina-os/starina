use alloc::borrow::Cow;
use alloc::vec::Vec;

use crate::header::Headers;
use crate::method::Method;

pub enum Body {
    Full(Vec<u8>),
}

pub enum BodyError {}
pub enum JsonBodyError {}

impl Body {
    pub async fn read(&self) -> Result<Option<Cow<'static, [u8]>>, BodyError> {
        todo!()
    }
}

pub struct Request {
    pub method: Method,
    pub headers: Headers,
    body: Option<Body>,
}

impl Request {
    pub fn new(method: Method, headers: Headers, body: Body) -> Self {
        Self {
            method,
            headers,
            body: Some(body),
        }
    }

    pub async fn body(&mut self) -> Option<Body> {
        self.body.take()
    }

    // pub async fn json<T>(&mut self) -> Result<T, JsonBodyError> {
    //     todo!()
    // }
}
