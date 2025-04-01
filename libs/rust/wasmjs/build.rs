use std::fs::File;
use std::io::Write;
use std::path::Path;

use wasmtime::Config;
use wasmtime::Engine;

pub fn main() {
    eprintln!("cargo:rerun-if-changed=build.rs");
    eprintln!("cargo:rerun-if-changed=app.wat");

    let mut config = Config::new();
    config
        .target("riscv64-unknown-unknown")
        .expect("failed to set target");

    config.memory_reservation(1024 * 1024);
    config.memory_guard_size(0);
    config.signals_based_traps(false);
    config.memory_init_cow(false);

    let engine = Engine::new(&config).unwrap();
    let precompiled = engine.precompile_module(include_bytes!("app.wat")).unwrap();

    let mut file = File::create("app.wasmtime").unwrap();
    file.write_all(&precompiled).unwrap();
}
