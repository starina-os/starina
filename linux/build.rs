use std::env;
use std::process::Command;

fn main() {
    if env::var_os("STARINA_RUN_SH").is_none() {
        // If STARINA_RUN_SH is not set, it's likely rust-analyzer triggered
        // this build script. Avoid running the build script in this case
        // not to drain your battery.
        println!("cargo:warning=Skipping build in rust-analyzer");
        return;
    }

    println!("cargo:rerun-if-changed=Makefile");
    println!("cargo:rerun-if-changed=linux.riscv64.config");
    println!("cargo:rerun-if-changed=linuxinit");
    println!("cargo:rerun-if-changed=catsay.go");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("failed to get manifest directory");

    let program = if cfg!(target_os = "macos") {
        "make"
    } else {
        "docker"
    };

    let status = Command::new(program)
        .arg("linux.elf")
        .current_dir(&manifest_dir)
        .status()
        .expect("failed to execute docker build");

    if !status.success() {
        panic!("Docker build failed");
    }
}
