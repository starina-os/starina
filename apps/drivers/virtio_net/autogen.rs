use starina::channel::Channel;
use starina::collections::HashMap;
// TODO: auto geenrate this file from app.toml
use starina::device_tree::DeviceTree;
use starina::iobus::IoBus;
use starina::prelude::*;
use starina::spec::AppImage;
use starina::spec::AppSpec;
use starina::spec::DeviceMatch;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::spec::ExportItem;
use starina::syscall::VsyscallPage;

use crate::App;
use crate::State;

#[derive(serde::Deserialize)]
pub struct Env {
    pub startup_ch: Channel,
    pub iobus: HashMap<String, IoBus>,
    pub device_tree: DeviceTree,
}

pub const APP_SPEC: AppSpec = AppSpec {
    name: "virtio_net",
    image: AppImage::Native { entrypoint },
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
    exports: &[ExportItem::Service {
        name: "device/ethernet",
    }],
};

extern "C" fn entrypoint(vsyscall: *const VsyscallPage) -> ! {
    starina::eventloop::app_loop::<Env, State, App>("virtio-net", vsyscall);
}
