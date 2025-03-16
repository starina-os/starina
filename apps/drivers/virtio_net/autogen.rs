// TODO: auto geenrate this file from app.toml
use starina::device_tree::DeviceTree;
use starina::spec::AppSpec;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::syscall::VsyscallPage;

use crate::App;

#[derive(serde::Deserialize)]
pub struct Env {
    pub device_tree: DeviceTree,
}

pub const APP_SPEC: AppSpec = AppSpec {
    env: &[EnvItem {
        name: "device_tree",
        ty: EnvType::DeviceTree {},
    }],
};

pub fn app_main(vsyscall: *const VsyscallPage) {
    starina::eventloop::app_loop::<Env, App>(vsyscall);
}
