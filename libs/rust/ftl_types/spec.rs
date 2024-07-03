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
    #[serde(rename = "boot/v0")]
    Boot(BootSpec),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSpec {
    pub depends: Vec<String>,
    pub provides: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootSpec {
    pub autostart_apps: Vec<String>,
}
