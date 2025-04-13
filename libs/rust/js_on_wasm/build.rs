use std::process::Command;

pub fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=app.js");
    println!("cargo:rerun-if-changed=starina.wit");

    Command::new("npm")
        .args([
            "run",
            "jco",
            "componentize",
            "--wit",
            "starina.wit",
            "-o",
            "app.component.wasm",
            "app.js",
            "-d",
            "all",
        ])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    let _ = std::fs::remove_dir_all("out");

    Command::new("npm")
        .args(["transpile", "app.component.wasm", "-o", "out"])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}
