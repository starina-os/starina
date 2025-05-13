use starina::prelude::*;
use starina_linux::BufferedStdin;
use starina_linux::BufferedStdout;

pub fn catsay(text: &str) {
    let stdin = BufferedStdin::new(text);
    let stdout = BufferedStdout::new();

    starina_linux::Command::new("catsay")
        .stdin(stdin)
        .stdout(stdout.clone())
        .spawn()
        .expect("failed to execute process");

    info!("stdout: {}", stdout.text().as_str());
}
