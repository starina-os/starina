pub enum DeviceMatch {
    Compatible(&'static str),
}

pub enum EnvType {
    DeviceTree { matches: &'static [DeviceMatch] },
    IoBusMap,
    Service { name: &'static str },
}

pub struct EnvItem {
    pub name: &'static str,
    pub ty: EnvType,
}

pub struct AppSpec {
    pub env: &'static [EnvItem],
}
