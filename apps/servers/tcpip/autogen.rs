use starina::channel::Channel;
use starina::spec::AppImage;
// TODO: auto geenrate this file from app.toml
use starina::spec::AppSpec;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::spec::ExportItem;
use starina::syscall::VsyscallPage;

use crate::App;
use crate::State;

#[derive(serde::Deserialize)]
pub struct Env {
    pub startup_ch: Channel,
    pub driver: Channel,
}

pub const APP_SPEC: AppSpec = AppSpec {
    name: "tcpip",
    image: AppImage::Native { entrypoint },
    env: &[EnvItem {
        name: "driver",
        ty: EnvType::Service {
            name: "device/ethernet",
        },
    }],
    exports: &[ExportItem::Service { name: "tcpip" }],
};

extern "C" fn entrypoint(vsyscall: *const VsyscallPage) -> ! {
    starina::eventloop::app_loop::<Env, State, App>("tcpip", vsyscall);
}
