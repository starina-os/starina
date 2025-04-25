use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;
use serde::Deserialize;
use serde::Serialize;

use crate::syscall::VsyscallPage;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EnvType {
    Service { service: String },
    DeviceTree { matches: Vec<String> },
    IoBusMap,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ExportItem {
    Service { service: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppSpec {
    pub name: String,
    pub env: HashMap<String, EnvType>,
    pub exports: Vec<ExportItem>,
}

/// `AppSpec`, in a pre-compiled form.
///
/// This is the format used by the kernel to load apps efficiently,
/// without parsing `app.toml` again at runtime.
#[derive(Debug)]
pub struct ParsedAppSpec {
    pub name: &'static str,
    pub env: &'static [ParsedEnvItem],
    pub exports: &'static [ParsedExportItem],
    pub entrypoint: extern "C" fn(vsyscall: *const VsyscallPage) -> !,
}

#[derive(Debug)]
pub enum ParsedDeviceMatch {
    Compatible(&'static str),
}

#[derive(Debug)]
pub struct ParsedEnvItem {
    pub name: &'static str,
    pub ty: ParsedEnvType,
}

#[derive(Debug)]
pub enum ParsedEnvType {
    DeviceTree {
        matches: &'static [ParsedDeviceMatch],
    },
    IoBusMap,
    Service {
        service: &'static str,
    },
}

#[derive(Debug)]
pub enum ParsedExportItem {
    Service { service: &'static str },
}
