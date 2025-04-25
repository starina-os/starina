pub fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    starina_build_sdk::autogen();
}
