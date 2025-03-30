use std::fs::File;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use wasmtime::Config;
use wasmtime::Engine;

pub fn main() {
    eprintln!("cargo:rerun-if-changed=build.rs");
    eprintln!("cargo:rerun-if-changed=app.js");

    let cmd = Command::new("porf")
        .arg("wasm")
        .arg("app.js")
        .arg("app.wasm")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    if !cmd.status.success() {
        eprintln!("cargo:warning=failed to compile app.js to WASM using porf");
    }

    let mut config = Config::new();
    config
        .target("riscv64-unknown-unknown")
        .expect("failed to set target");

    config.memory_reservation(1024 * 1024);
    config.memory_guard_size(0);
    config.signals_based_traps(false);

    let engine = Engine::new(&config).unwrap();
    let precompiled = engine
        .precompile_module(include_bytes!("app.wasm"))
        .unwrap();

    let mut file = File::create("app.precompiled").unwrap();
    file.write_all(&precompiled).unwrap();
}
