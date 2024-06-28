use alloc::string::String;
use alloc::vec::Vec;

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
pub enum Ty {
    Int32,
    Handle,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
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
pub struct StringField {
    pub len: u16,
    pub offset: u16,
}
