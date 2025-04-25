use std::fs;
use std::path::PathBuf;

pub fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=app.wat");
    println!("cargo:rerun-if-changed=app.toml");

    // Convert WAT to WASM
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let wat = fs::read(manifest_dir.join("app.wat")).expect("failed to read wat file");
    let wasm = wasmer::wat2wasm(&wat).expect("failed to convert wat to wasm");
    fs::write(manifest_dir.join("app.wasm"), wasm).expect("failed to write wasm file");

    // Auto-generate autogen.rs from app.toml
    starina_build_sdk::autogen();
}
