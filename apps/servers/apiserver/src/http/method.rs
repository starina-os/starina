use core::fmt;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Patch,
    Delete,
    Options,
    Trace,
    Connect,
}

pub struct InvalidMethodError;

impl Method {
    pub const fn from_bytes(bytes: &[u8]) -> Result<Self, InvalidMethodError> {
        match bytes {
            b"GET" => Ok(Method::Get),
            b"HEAD" => Ok(Method::Head),
            b"POST" => Ok(Method::Post),
            b"PUT" => Ok(Method::Put),
            b"PATCH" => Ok(Method::Patch),
            b"DELETE" => Ok(Method::Delete),
            b"OPTIONS" => Ok(Method::Options),
            b"TRACE" => Ok(Method::Trace),
            b"CONNECT" => Ok(Method::Connect),
            _ => Err(InvalidMethodError),
        }
    }
}

impl fmt::Debug for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Method::Get => write!(f, "GET"),
            Method::Head => write!(f, "HEAD"),
            Method::Post => write!(f, "POST"),
            Method::Put => write!(f, "PUT"),
            Method::Patch => write!(f, "PATCH"),
            Method::Delete => write!(f, "DELETE"),
            Method::Options => write!(f, "OPTIONS"),
            Method::Trace => write!(f, "TRACE"),
            Method::Connect => write!(f, "CONNECT"),
        }
    }
}
