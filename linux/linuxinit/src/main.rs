use std::fs::File;
use std::fs::OpenOptions;
use std::net::SocketAddr;

use nix::mount::MsFlags;
use nix::mount::mount;
use nix::sys::reboot::RebootMode;
use nix::sys::reboot::reboot;
use serde::Deserialize;
use serde::Serialize;
use tokio::process::Command;

#[derive(Debug, Serialize, Deserialize)]
struct CommandJson {
    program: String,
    args: Vec<String>,
}

#[tokio::main]
async fn main() {
    eprintln!("[linuxinit] starting");

    eprintln!("[linuxinit] mounting sysfs");
    mount(
        Some("sysfs"),
        "/sys",
        Some("sysfs"),
        MsFlags::empty(),
        None as Option<&str>,
    )
    .expect("failed to mount sysfs");

    eprintln!("[linuxinit] mounting procfs");
    mount(
        Some("proc"),
        "/proc",
        Some("proc"),
        MsFlags::empty(),
        None as Option<&str>,
    )
    .expect("failed to mount procfs");

    eprintln!("[linuxinit] mounting tmpfs");
    mount(
        Some("tmpfs"),
        "/tmp",
        Some("tmpfs"),
        MsFlags::empty(),
        None as Option<&str>,
    )
    .expect("failed to mount tmpfs");

    eprintln!("[linuxinit] mounting virtio-fs");
    mount(
        Some("virtfs"),
        "/virtfs",
        Some("virtiofs"),
        MsFlags::empty(),
        None as Option<&str>,
    )
    .expect("failed to mount virtio-fs");

    eprintln!("[linuxinit] opening /virtfs files");
    let command_json_file = File::open("/virtfs/command").expect("failed to open /virtfs/command");

    let stdin_file = OpenOptions::new()
        .read(true)
        .open("/virtfs/stdin")
        .expect("failed to open /virtfs/stdin");

    let stdout_file = OpenOptions::new()
        .write(true)
        .open("/virtfs/stdout")
        .expect("failed to open /virtfs/stdout");

    let command_json: CommandJson =
        serde_json::from_reader(command_json_file).expect("failed to parse /virtfs/command");

    eprintln!("[linuxinit] starting command: {}", command_json.program);
    let mut cmd = Command::new(&command_json.program)
        .args(&command_json.args)
        .stdin(stdin_file)
        .stdout(stdout_file)
        .spawn()
        .expect("failed to spawn command");

    let exit_status = cmd.wait().await.expect("failed to wait on command");
    eprintln!("command exited with status: {:?}", exit_status);

    eprintln!("[linuxinit] shuting down ...");
    reboot(RebootMode::RB_HALT_SYSTEM).unwrap();
}
