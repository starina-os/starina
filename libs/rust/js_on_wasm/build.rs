use std::process::Command;

fn run_with_pnpm(args: &[&str]) {
    let mut command = Command::new("pnpm");
    command.args(args);
    let status = command.status().expect("failed to execute pnpm");
    if !status.success() {
        panic!(
            "pnpm command failed with status {}: {}",
            status,
            args.join(" ")
        );
    }
}

pub fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=app.js");
    println!("cargo:rerun-if-changed=starina.wit");

    // let _ = std::fs::remove_dir_all("out");

    // run_with_pnpm(&[
    //     "jco",
    //     "componentize",
    //     "--aot",
    //     "--wit",
    //     "starina.wit",
    //     "-o",
    //     "app.component.wasm",
    //     "app.js",
    //     "-d",
    //     "all",
    // ]);

    // run_with_pnpm(&["jco", "transpile", "app.component.wasm", "-o", "out"]);
}
