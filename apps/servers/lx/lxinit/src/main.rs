use nix::mount::MsFlags;
use nix::mount::mount;

fn main() {
    println!("");
    println!("");
    println!("");
    println!("Hello World from lxinit!");
    println!("");
    println!("");
    println!("");

    mount(
        Some("virtfs"),
        "/virtfs",
        Some("virtiofs"),
        MsFlags::empty(),
        None as Option<&str>,
    )
    .expect("failed to mount virtio-fs");

    // List files in /virtfs
    println!("Listing files in /virtfs:");
    let files = std::fs::read_dir("/virtfs").expect("failed to read /virtfs");
    for file in files {
        println!("{}", file.unwrap().path().display());
    }
}
