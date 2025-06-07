#![no_std]

use starina::prelude::*;
use starina::spec::AppSpec;

pub const SPEC: AppSpec = AppSpec {
    name: "hello_swift",
    env: &[],
    exports: &[],
    main,
};

// External Swift function
unsafe extern "C" {
    fn swift_hello_world();
}

// Function to be called from Swift
#[unsafe(no_mangle)]
pub extern "C" fn rust_print_from_swift() {
    info!("Hello from Swift running on Starina OS! 🦉⚡");
}

fn main(_env_json: &[u8]) {
    info!("Hello, Swift! 🚀");
    info!("Welcome to Starina OS!");
    info!("This is a hello world app written in Rust for Starina.");

    // Call the Swift function
    unsafe {
        swift_hello_world();
    }

    info!("Swift integration complete!");
}
