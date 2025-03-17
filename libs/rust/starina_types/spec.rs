pub enum DeviceMatch {
    Compatible(&'static str),
}

pub enum EnvType {
    DeviceTree { matches: &'static [DeviceMatch] },
}

pub struct EnvItem {
    pub name: &'static str,
    pub ty: EnvType,
}

pub struct AppSpec {
    pub env: &'static [EnvItem],
}
