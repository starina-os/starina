use std::env;
use std::fs::File;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if env::var_os("STARINA_RUN_SH").is_none() {
        // If STARINA_RUN_SH is not set, it's likely rust-analyzer triggered
        // this build script. Avoid running the build script in this case
        // not to drain your battery.
        println!("cargo:warning=Skipping build in rust-analyzer");
        return;
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let tarball_path = Path::new(&out_dir).join("container.tar");
    let squashfs_path = Path::new(&out_dir).join("container.squashfs");

    std::fs::remove_file(&tarball_path).unwrap_or_default();
    std::fs::remove_file(&squashfs_path).unwrap_or_default();

    let docker_status: std::process::ExitStatus = Command::new("docker")
        .arg("run")
        .arg("--platform")
        .arg("linux/riscv64")
        .arg("--rm")
        .arg("riscv64/alpine:latest")
        .arg("tar")
        .arg("vcf")
        .arg("-")
        .arg("--exclude=./dev/**")
        .arg("--exclude=./proc/**")
        .arg("--exclude=./sys/**")
        .arg("--exclude=/tmp")
        .arg(".")
        .arg("-C")
        .arg("/")
        .stdout(Stdio::from(File::create(&tarball_path).unwrap()))
        .status()
        .expect("failed to docker run");

    if !docker_status.success() {
        panic!("Docker extraction failed");
    }

    let mksquashfs_status = Command::new("mksquashfs")
        .arg("-")
        .arg(&squashfs_path)
        .arg("-tar")
        .arg("-comp")
        .arg("lz4")
        .stdin(Stdio::from(File::open(&tarball_path).unwrap()))
        .status()
        .expect("failed to mksquashfs");

    if !mksquashfs_status.success() {
        panic!("mksquashfs failed");
    }
}
