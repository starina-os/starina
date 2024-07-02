use alloc::string::String;
use alloc::vec::Vec;
use core::str;
use core::str::Utf8Error;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdlFile {
    pub protocols: Vec<Protocol>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Protocol {
    pub name: String,
    pub rpcs: Vec<Rpc>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct BytesTy {
        capacity: usize
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Ty {
    Int32,
    Handle,
    Bytes { capacity: usize },
    String { capacity: usize },
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(flatten)]
    pub ty: Ty,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub fields: Vec<Field>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rpc {
    pub name: String,
    pub request: Message,
    pub response: Message,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TooManyItemsError;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
pub struct BytesField<const CAP: usize> {
    len: u16,
    data: [u8; CAP],
}

impl<const CAP: usize> BytesField<CAP> {
    pub const fn new(data: [u8; CAP], len: u16) -> Self {
        Self {
            len,
            data,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }
}

impl<const CAP: usize> TryFrom<&[u8]> for BytesField<CAP> {
    type Error = TooManyItemsError;

    fn try_from(value: &[u8]) -> Result<BytesField<CAP>, TooManyItemsError> {
        debug_assert!(CAP <= u16::MAX as usize); // CAP must fit in u16

        if value.len() > CAP {
            return Err(TooManyItemsError);
        }

        // Zeroing the remaining part is very important to avoid leak
        // information - the kernel will keep copying the whole CAP bytes
        // regardless of the length!
        let mut data: [u8; CAP] = [0; CAP];

        data[..value.len()].copy_from_slice(value);

        Ok(BytesField::new(data, value.len() as u16))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(transparent)]
pub struct StringField<const CAP: usize>(BytesField<CAP>);

impl<const CAP: usize> StringField<CAP> {
    pub fn to_str(&self) -> Result<&str, Utf8Error> {
        str::from_utf8(self.0.as_slice())
    }
}

impl<const CAP: usize> TryFrom<&str> for StringField<CAP> {
    type Error = TooManyItemsError;

    fn try_from(value: &str) -> Result<StringField<CAP>, TooManyItemsError> {
        let bytes = BytesField::try_from(value.as_bytes())?;
        Ok(StringField(bytes))
    }
}
