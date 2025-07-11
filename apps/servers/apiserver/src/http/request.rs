use starina::prelude::*;

use super::Headers;
use super::Method;

#[derive(Debug, PartialEq, Eq)]
pub enum Body {
    Full(Vec<u8>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub headers: Headers,
    pub body: Body,
}
