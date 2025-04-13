use starina::channel::Channel;
// TODO: auto geenrate this file from app.toml
use starina::spec::AppSpec;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::spec::ExportItem;
use starina::syscall::VsyscallPage;

use crate::App;

#[derive(serde::Deserialize)]
pub struct Env {
    pub startup_ch: Channel,
    pub driver: Channel,
}

pub const APP_SPEC: AppSpec = AppSpec {
    env: &[EnvItem {
        name: "driver",
        ty: EnvType::Service {
            name: "device/ethernet",
        },
    }],
    exports: &[ExportItem::Service { name: "tcpip" }],
};

pub fn app_main(vsyscall: *const VsyscallPage) {
    starina::eventloop::app_loop::<Env, App>("tcpip", vsyscall);
}
