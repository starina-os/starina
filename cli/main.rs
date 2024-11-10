use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use clap::Parser;

#[macro_use]
mod print;

mod commands;
mod make;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: commands::Command,
}

fn look_for_cli_dir() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().unwrap();
    loop {
        let cargo_toml_path = dir.join("cli").join("Cargo.toml");
        let cargo_toml = std::fs::read_to_string(&cargo_toml_path);
        if let Ok(cargo_toml) = cargo_toml {
            if cargo_toml.contains("name = \"starina-cli\"") {
                return Some(cargo_toml_path);
            }
        }

        if !dir.pop() {
            return None;
        }
    }
}

fn main() {
    let exe = std::env::args().nth(0).unwrap();
    if exe == "sx" {
        // Try to build and run the local CLI.
        if let Some(cargo_toml_path) = look_for_cli_dir() {
            println!("Running the local CLI at: {}", cargo_toml_path.display());
            let err = Command::new("cargo")
                .args(["run", "--bin", "sx", "--manifest-path"])
                .arg(cargo_toml_path)
                .args(std::env::args().skip(1))
                .exec();

            panic!("failed to run the local CLI: {}", err);
        }
    }

    let args = Cli::parse();
    if let Err(err) = commands::run_command(args.command) {
        error!("{}", err);
    }
}
