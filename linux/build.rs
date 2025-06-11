use std::env;
use std::process::Command;

fn main() {
    if env::var_os("STARINA_RUN_SH").is_none() {
        // If STARINA_RUN_SH is not set, it's likely rust-analyzer triggered
        // this build script. Avoid running the build script in this case
        // not to drain your battery.
        println!("cargo:warning=Skipping build in rust-analyzer");
        return;
    }

    let docker_info = Command::new("docker").arg("info").output();
    if docker_info.is_err() || !docker_info.unwrap().status.success() {
        panic!("Docker is not available. Please start Docker to continue.");
    }

    println!("cargo:rerun-if-changed=Dockerfile");
    println!("cargo:rerun-if-changed=build-linux.sh");
    println!("cargo:rerun-if-changed=linux.riscv64.config");
    println!("cargo:rerun-if-changed=linuxinit");
    println!("cargo:rerun-if-changed=catsay.go");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("failed to get manifest directory");
    let build_status = Command::new("docker")
        .arg("build")
        .arg("--progress=plain")
        .arg("--build-arg")
        .arg("BUILDKIT_INLINE_CACHE=1")
        .arg("-t")
        .arg("starina-linux")
        .arg(".")
        .current_dir(&manifest_dir)
        .env("DOCKER_BUILDKIT", "1")
        .status()
        .expect("failed to execute docker build");

    if !build_status.success() {
        panic!("Docker build failed");
    }

    let run_status = Command::new("docker")
        .arg("run")
        .arg("--rm")
        .arg("-v")
        .arg(format!("{manifest_dir}:/linux"))
        .arg("starina-linux")
        .current_dir(&manifest_dir)
        .status()
        .expect("failed to execute docker run");

    if !run_status.success() {
        panic!("Docker run failed");
    }
}
