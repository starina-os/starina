use crate::syscall::VsyscallPage;

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

pub enum ExportItem {
    Service { name: &'static str },
}

pub enum AppImage {
    Native {
        entrypoint: extern "C" fn(vsyscall: *const VsyscallPage) -> !,
    },
    Wasm {
        wasm: &'static [u8],
    },
}

pub struct AppSpec {
    pub name: &'static str,
    pub image: AppImage,
    pub env: &'static [EnvItem],
    pub exports: &'static [ExportItem],
}
