use starina::prelude::*;

use super::Headers;
use super::Method;

#[derive(Debug, PartialEq, Eq)]
pub enum Body {
    Full(Vec<u8>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Query {
    params: Vec<(String, String)>,
}

impl Query {
    pub fn new() -> Self {
        Self {
            params: Vec::new(),
        }
    }

    pub fn from_str(query: &str) -> Self {
        let mut params = Vec::new();
        
        if !query.is_empty() {
            for param in query.split('&') {
                if let Some((key, value)) = param.split_once('=') {
                    params.push((key.to_string(), value.to_string()));
                }
            }
        }
        
        Self { params }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub query: Query,
    pub headers: Headers,
    pub body: Body,
}
