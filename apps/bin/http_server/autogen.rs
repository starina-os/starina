use starina::channel::Channel;
// TODO: auto geenrate this file from app.toml
use starina::spec::AppSpec;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::syscall::VsyscallPage;

use crate::App;

#[derive(serde::Deserialize)]
pub struct Env {
    pub tcpip: Channel,
    // pub listen_host: String,
    // pub listen_port: u16,
}

pub const APP_SPEC: AppSpec = AppSpec {
    env: &[EnvItem {
        name: "tcpip",
        ty: EnvType::Service { name: "tcpip" },
    }],
    exports: &[],
};

pub fn app_main(vsyscall: *const VsyscallPage) {
    starina::eventloop::app_loop::<Env, App>("http_server", vsyscall);
}
