use crate::environ::Environ;

#[derive(Debug)]
pub struct AppSpec {
    pub name: &'static str,
    pub env: &'static [EnvItem],
    pub exports: &'static [ExportItem],
    pub main: fn(env: Environ),
}

#[derive(Debug)]
pub enum DeviceMatch {
    Compatible(&'static str),
}

#[derive(Debug)]
pub struct EnvItem {
    pub name: &'static str,
    pub ty: EnvType,
}

#[derive(Debug)]
pub enum EnvType {
    DeviceTree { matches: &'static [DeviceMatch] },
    Service { service: &'static str },
}

#[derive(Debug)]
pub enum ExportItem {
    Service { service: &'static str },
}
