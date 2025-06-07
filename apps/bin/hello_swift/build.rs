use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=hello.swift");
    println!("cargo:rerun-if-changed=bridging_header.h");

    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Target triple for RISC-V 64-bit (Starina's target)
    let target_triple = "riscv64-none-none-eabi";

    let swift_source = Path::new(&manifest_dir).join("hello.swift");
    let bridging_header = Path::new(&manifest_dir).join("bridging_header.h");
    let output_object = Path::new(&out_dir).join("hello.o");

    // Check if Swift source exists before trying to compile
    if !swift_source.exists() {
        panic!(
            "cargo:warning=Swift source file not found: {}",
            swift_source.display()
        );
        return;
    }

    // Compile Swift code using swiftc
    let mut cmd = Command::new("swiftc");
    cmd.arg("-enable-experimental-feature")
        .arg("Embedded")
        .arg("-wmo")
        .arg("-import-bridging-header")
        .arg(&bridging_header)
        .arg("-target")
        .arg(target_triple)
        .arg(&swift_source)
        .arg("-c")
        .arg("-o")
        .arg(&output_object);

    let output = cmd.output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                println!("cargo:warning=Swift compilation failed:");
                println!(
                    "cargo:warning=stdout: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
                println!(
                    "cargo:warning=stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                return;
            }

            // Tell Cargo to link the compiled Swift object
            println!("cargo:rustc-link-arg={}", output_object.display());
        }
        Err(e) => {
            println!("cargo:warning=Failed to execute swiftc: {}", e);
            return;
        }
    }
}
