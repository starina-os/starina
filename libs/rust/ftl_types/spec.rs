use alloc::string::String;
use alloc::vec::Vec;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecFile {
    pub name: String,
    #[serde(flatten)]
    pub spec: Spec,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "spec")]
pub enum Spec {
    #[serde(rename = "app/v0")]
    App(AppSpec),
    #[serde(rename = "interface/v0")]
    Interface(InterfaceSpec),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceTree {
    pub compatible: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Depend {
    Service { interface: String },
    Device { device_tree: Option<DeviceTree> },
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependWithName {
    pub name: String,
    #[serde(flatten)]
    pub depend: Depend,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSpec {
    pub depends: Vec<DependWithName>,
    pub provides: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceSpec {
    pub messages: Vec<Message>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// The message name.
    pub name: String,
    /// The message description.
    pub description: Option<String>,
    /// The message type.
    #[serde(rename = "type")]
    pub ty: MessageType,
    /// The channel context.
    pub context: String,
    /// Who sends the message.
    pub origin: Option<MessageOrigin>,
    /// The message fields.
    pub params: Vec<MessageField>,
    /// The return message fields. Only valid in [`MessageType::Call`].
    pub returns: Option<Vec<MessageField>>,
}

/// How the message is sent.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    /// Send and then receive the response message.
    Call,
    /// An one-way message.
    Push,
}

/// Who sends the message.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageOrigin {
    /// The message is sent from client, to server.
    Client,
    /// The message is sent from server, to client.
    Server,
    /// Both client/server may send the message, in other words, no
    /// client/server distinction.
    Both,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageField {
    pub name: String,
    #[serde(flatten)]
    pub ty: MessageFieldType,
    pub help: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum MessageFieldType {
    UInt8,
    UInt16,
    UInt32,
    Int8,
    Int16,
    Int32,
    Channel,
    Bytes { capacity: usize },
    String { capacity: usize },
}
