use core::str::from_utf8;

use starina::prelude::*;
use starina_linux::BufferedStdin;
use starina_linux::BufferedStdout;
use starina_linux::Port;

pub fn catsay(text: &str) {
    let stdin = BufferedStdin::new(text);
    let stdout = BufferedStdout::new();

    starina_linux::Command::new("/bin/catsay")
        .stdin(stdin)
        .stdout(stdout.clone())
        .port(Port::Tcp {
            host: 8080,
            guest: 8080,
        })
        .spawn()
        .expect("failed to execute process");

    info!("{}", from_utf8(&stdout.buffer()).unwrap());
}
