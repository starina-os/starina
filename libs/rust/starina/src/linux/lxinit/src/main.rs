use std::fs::File;
use std::io::Read;

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

    // Open /virtfs/test.txt
    let mut file = File::open("/virtfs/test.txt").expect("failed to open /virtfs/test.txt");
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)
        .expect("failed to read /virtfs/test.txt");

    let contents = match std::str::from_utf8(&contents) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "failed to convert /virtfs/test.txt to UTF-8: {:?}: {:02x?}",
                e, &contents
            );
            return;
        }
    };

    println!("--------------------------------");
    println!("/virtfs/test.txt: \"{}\"", contents);
    println!("--------------------------------");

    // List files in /virtfs
    // println!("Listing files in /virtfs:");
    // let files = std::fs::read_dir("/virtfs").expect("failed to read /virtfs");
    // for file in files {
    //     println!("{}", file.unwrap().path().display());
    // }
}
