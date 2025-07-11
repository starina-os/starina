use core::convert::Infallible;
use core::fmt;
use core::ops::Deref;

use starina::borrow::Cow;
use starina::collections::HashMap;
use starina::prelude::*;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum InvalidNameError {
    #[error("Header name is empty")]
    Empty,
    #[error("Invalid byte in header name: {0}")]
    InvalidByte(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HeaderName(Cow<'static, str>);

impl HeaderName {
    pub const CONTENT_TYPE: Self = Self::from_static("content-type");
    pub const SERVER: Self = Self::from_static("server");

    pub const fn from_static(name: &'static str) -> Self {
        if name.is_empty() {
            panic!("empty header name");
        }

        // Use a C-style while loop due to rustc's constraint in a const fn.
        let bytes = name.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i];
            let is_valid = c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_';
            if !is_valid {
                panic!(
                    "invalid header name (only lowercase letters, digits, '-' and '_' are allowed)"
                );
            }
            i += 1;
        }

        HeaderName(Cow::Borrowed(name))
    }

    pub fn parse(name: &str) -> Result<Self, InvalidNameError> {
        if name.is_empty() {
            return Err(InvalidNameError::Empty);
        }

        let mut s = String::with_capacity(name.len());
        for c in name.as_bytes() {
            let is_valid = c.is_ascii_alphanumeric() || *c == b'-' || *c == b'_';
            if !is_valid {
                return Err(InvalidNameError::InvalidByte(*c));
            }
            s.push(c.to_ascii_lowercase() as char);
        }

        Ok(HeaderName(Cow::Owned(s)))
    }
}

impl fmt::Display for HeaderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for HeaderName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<&str> for HeaderName {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq_ignore_ascii_case(other)
    }
}

impl TryFrom<&str> for HeaderName {
    type Error = InvalidNameError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        HeaderName::parse(value)
    }
}

impl From<Infallible> for InvalidNameError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum InvalidValueError {
    #[error("Header value is empty")]
    Empty,
    #[error("Invalid character in header value: {0}")]
    InvalidByte(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderValue(String);

impl fmt::Display for HeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for HeaderValue {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<str> for HeaderValue {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl TryFrom<&str> for HeaderValue {
    type Error = InvalidValueError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(InvalidValueError::Empty);
        }

        let mut s = String::with_capacity(value.len());
        for c in value.chars() {
            if c == '\r' || c == '\n' {
                return Err(InvalidValueError::InvalidByte(c));
            }
            s.push(c);
        }

        Ok(HeaderValue(s))
    }
}

#[derive(Debug, Error)]
pub enum InsertError {
    #[error("Header already exists")]
    AlreadyExists,
    #[error("Invalid header name: {0}")]
    InvalidName(#[from] InvalidNameError),
    #[error("Invalid header value: {0}")]
    InvalidValue(#[from] InvalidValueError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Headers {
    entries: HashMap<HeaderName, Vec<HeaderValue>>,
}

impl Headers {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert<K, V>(&mut self, name: K, value: V) -> Result<(), InsertError>
    where
        K: TryInto<HeaderName>,
        K::Error: Into<InvalidNameError>,
        V: TryInto<HeaderValue, Error = InvalidValueError>,
    {
        let name = name
            .try_into()
            .map_err(|e| InsertError::InvalidName(e.into()))?;
        let value = value.try_into().map_err(InsertError::InvalidValue)?;
        if self.entries.contains_key(&name) {
            return Err(InsertError::AlreadyExists);
        }

        self.entries.insert(name, vec![value]);
        Ok(())
    }

    pub fn get<K>(&self, name: K) -> Result<Option<&HeaderValue>, InvalidNameError>
    where
        K: TryInto<HeaderName, Error = InvalidNameError>,
    {
        let name = name.try_into()?;
        let value = self.entries.get(&name).and_then(|v| v.first());
        Ok(value)
    }

    pub fn append<K, V>(&mut self, name: K, value: V) -> Result<(), InsertError>
    where
        K: TryInto<HeaderName, Error = InvalidNameError>,
        V: TryInto<HeaderValue, Error = InvalidValueError>,
    {
        let name = name
            .try_into()
            .map_err(|e| InsertError::InvalidName(e.into()))?;
        let value = value.try_into().map_err(InsertError::InvalidValue)?;
        self.entries
            .entry(name)
            .or_insert_with(Vec::new)
            .push(value);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.entries
            .iter()
            .flat_map(|(name, values)| values.iter().map(move |value| (name, value)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_name() {
        let name = HeaderName::from_static("content-type");
        assert_eq!(name, "content-type");
    }

    #[test]
    fn test_header_name_parse() {
        let name = HeaderName::parse("X-Custom-Header").unwrap();
        assert_eq!(name, "x-custom-header");
    }

    #[test]
    fn test_header_insert() {
        let mut headers = Headers::new();
        headers.insert("X-Custom-Header", "value").unwrap();
        assert_eq!(headers.get("x-custom-header").unwrap().unwrap(), "value");
    }
}
