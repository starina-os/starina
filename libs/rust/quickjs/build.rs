pub fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    cc::Build::new()
        .target("riscv64gc-unknown-none-elf") // TODO: remove this hardcoded target
        .file("quickjs-amalgam.c")
        .define("EMSCRIPTEN", None)
        .warnings(false)
        .compile("quickjs");
}
