use std::fs::File;
use std::fs::OpenOptions;

use nix::mount::MsFlags;
use nix::mount::mount;
use nix::sys::reboot::RebootMode;
use nix::sys::reboot::reboot;
use nix::unistd::chdir;
use nix::unistd::chroot;
use serde::Deserialize;
use serde::Serialize;
use tokio::process::Command;

mod loopback;
use loopback::LoopDevice;

#[derive(Debug, Serialize, Deserialize)]
struct CommandJson {
    program: String,
    args: Vec<String>,
}

#[tokio::main]
async fn main() {
    eprintln!("[linuxinit] starting");

    eprintln!("[linuxinit] mounting devtmpfs");
    mount(
        Some("devtmpfs"),
        "/dev",
        Some("devtmpfs"),
        MsFlags::empty(),
        None as Option<&str>,
    )
    .expect("failed to mount devtmpfs");

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

    eprintln!("[linuxinit] creating /containerfs directory");
    std::fs::create_dir_all("/containerfs").expect("failed to create /containerfs directory");

    eprintln!("[linuxinit] creating loop device");
    let mut loopback = LoopDevice::new(0);
    loopback
        .attach("/virtfs/rootfs")
        .expect("failed to attach squashfs to loop device");

    eprintln!("[linuxinit] mounting /containerfs");
    mount(
        Some(loopback.device_path().to_str().unwrap()),
        "/containerfs",
        Some("squashfs"),
        MsFlags::MS_RDONLY,
        None as Option<&str>,
    )
    .expect("failed to mount /containerfs");

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

    eprintln!("[linuxinit] preparing containerized environment");

    eprintln!("[linuxinit] mounting essential filesystems in container");

    if std::path::Path::new("/containerfs/proc").exists() {
        mount(
            Some("proc"),
            "/containerfs/proc",
            Some("proc"),
            MsFlags::empty(),
            None as Option<&str>,
        )
        .expect("failed to mount proc in container");
    }

    if std::path::Path::new("/containerfs/sys").exists() {
        mount(
            Some("sysfs"),
            "/containerfs/sys",
            Some("sysfs"),
            MsFlags::empty(),
            None as Option<&str>,
        )
        .expect("failed to mount sys in container");
    }

    if std::path::Path::new("/containerfs/tmp").exists() {
        mount(
            Some("tmpfs"),
            "/containerfs/tmp",
            Some("tmpfs"),
            MsFlags::empty(),
            None as Option<&str>,
        )
        .expect("failed to mount tmp in container");
    }

    if std::path::Path::new("/containerfs/dev").exists() {
        mount(
            Some("devtmpfs"),
            "/containerfs/dev",
            Some("devtmpfs"),
            MsFlags::empty(),
            None as Option<&str>,
        )
        .expect("failed to mount dev in container");
    }

    eprintln!("[linuxinit] changing root to containerfs");
    chdir("/containerfs").expect("failed to chdir to /containerfs");
    chroot("/containerfs").expect("failed to chroot to /containerfs");
    chdir("/").expect("failed to chdir to / after chroot");

    eprintln!(
        "[linuxinit] starting command in container: {}",
        command_json.program
    );
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
