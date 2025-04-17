use starina::channel::Channel;
use starina::spec::AppImage;
// TODO: auto geenrate this file from app.toml
use starina::spec::AppSpec;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::syscall::VsyscallPage;

use crate::App;
use crate::State;

#[derive(serde::Deserialize)]
pub struct Env {
    pub tcpip: Channel,
    // pub listen_host: String,
    // pub listen_port: u16,
}

pub const APP_SPEC: AppSpec = AppSpec {
    name: "http_server",
    image: AppImage::Native { entrypoint },
    env: &[EnvItem {
        name: "tcpip",
        ty: EnvType::Service { name: "tcpip" },
    }],
    exports: &[],
};

extern "C" fn entrypoint(vsyscall: *const VsyscallPage) -> ! {
    starina::eventloop::app_loop::<Env, State, App>("http_server", vsyscall);
}
