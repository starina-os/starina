#![no_std]

use core::str::from_utf8;

use serde::Deserialize;
use starina::channel::Channel;
use starina::prelude::*;
use starina::spec::AppSpec;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::spec::ExportItem;
use starina_linux::BufferedStdin;
use starina_linux::BufferedStdout;
use starina_linux::ContainerImage;
use starina_linux::Port;

const CONTAINER_SQUASHFS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/container.squashfs"));

pub const SPEC: AppSpec = AppSpec {
    name: "linuxrun",
    env: &[EnvItem {
        name: "tcpip",
        ty: EnvType::Service { service: "tcpip" },
    }],
    exports: &[ExportItem::Service {
        service: "linuxrun",
    }],
    main,
};

#[derive(Debug, Deserialize)]
struct Env {
    pub tcpip: Channel,
}

fn main(env_json: &[u8]) {
    let env: Env = serde_json::from_slice(env_json).unwrap();

    const TEXT: &str = "I'm a teapot!";
    let stdin = BufferedStdin::new(TEXT);
    let stdout = BufferedStdout::new();

    starina_linux::Command::new("/bin/uname")
        .arg("-a")
        .image(ContainerImage::Static(CONTAINER_SQUASHFS))
        .stdin(stdin)
        .stdout(stdout.clone())
        .port(Port::Tcp {
            host: 8080,
            guest: 8080,
        })
        .spawn(env.tcpip)
        .expect("failed to execute process");

    info!(
        "\x1b[1;32m{}\x1b[0m",
        from_utf8(&stdout.buffer()).unwrap().trim_ascii_end()
    );
}
