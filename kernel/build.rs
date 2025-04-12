use std::env::{self};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "riscv64" => "riscv64",
        _ => panic!("Unsupported architecture"),
    };

    generate_linker_script(&out_dir, arch);
}

fn generate_linker_script(out_dir: &Path, arch: &str) {
    let template_path = PathBuf::from(format!("arch/{}/kernel.ld", arch));
    let dest_path = out_dir.join("linker_script.ld");
    let ld = fs::read_to_string(&template_path).expect("failed to read linker script");
    // TODO: Apply a template engine here.
    fs::write(&dest_path, ld).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", template_path.display());
    println!("cargo:rustc-link-arg-bin=kernel=-T{}", dest_path.display());
    println!("cargo:rustc-link-arg-bin=kernel=-Map=kernel.map");
}
