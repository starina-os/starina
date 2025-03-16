pub enum EnvType {
    DeviceTree {},
}

pub struct EnvItem {
    pub name: &'static str,
    pub ty: EnvType,
}

pub struct AppSpec {
    pub env: &'static [EnvItem],
}
