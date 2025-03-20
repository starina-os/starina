use starina::collections::HashMap;
// TODO: auto geenrate this file from app.toml
use starina::device_tree::DeviceTree;
use starina::iobus::IoBus;
use starina::prelude::*;
use starina::spec::AppSpec;
use starina::spec::DeviceMatch;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::syscall::VsyscallPage;

use crate::App;

#[derive(serde::Deserialize)]
pub struct Env {
    pub iobus: HashMap<String, IoBus>,
    pub device_tree: DeviceTree,
}

pub const APP_SPEC: AppSpec = AppSpec {
    env: &[
        EnvItem {
            name: "device_tree",
            ty: EnvType::DeviceTree {
                matches: &[DeviceMatch::Compatible("virtio,mmio")],
            },
        },
        EnvItem {
            name: "iobus",
            ty: EnvType::IoBusMap,
        },
    ],
};

pub fn app_main(vsyscall: *const VsyscallPage) {
    starina::eventloop::app_loop::<Env, App>(vsyscall);
}
