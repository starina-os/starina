use std::env;
use std::process::Command;

fn main() {
    if env::var_os("STARINA_MAKEFILE").is_none() {
        // If STARINA_MAKEFILE is not set, it's likely rust-analyzer triggered
        // this build script. Avoid running the build script in this case
        // not to drain your battery.
        println!("cargo:warning=Skipping build in rust-analyzer");
        return;
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Makefile");
    println!("cargo:rerun-if-changed=linux.riscv64.config");
    println!("cargo:rerun-if-changed=bootd");

    let program = if cfg!(target_os = "macos") {
        "/opt/homebrew/bin/gmake"
    } else {
        "make"
    };

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("failed to get manifest directory");
    let num_cpus = std::thread::available_parallelism().unwrap();
    let status = Command::new(program)
        .current_dir(&manifest_dir)
        .arg(format!("-j{num_cpus}"))
        .env_clear()
        // Apparently Cargo propagates some environment variables and confuses
        // another Cargo to be invoked in make.
        .env("PATH", env::var_os("PATH").unwrap())
        .env("HOME", env::var_os("HOME").unwrap())
        .env("MAKEFLAGS", env::var_os("MAKEFLAGS").unwrap_or("".into()))
        .status()
        .expect("failed to build Linux");

    if !status.success() {
        panic!("Linux build failed");
    }
}
