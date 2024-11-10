use std::env::current_dir;
use std::env::set_current_dir;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
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

fn move_to_project_dir() -> Result<()> {
    // Look for the directory with Makefile.
    let mut dir = current_dir()?;
    loop {
        if dir.join("Makefile").exists() && dir.join("Cargo.lock").exists() {
            set_current_dir(&dir)
                .with_context(|| format!("failed to chdir to {}", dir.display()))?;
            return Ok(());
        }

        if !dir.pop() {
            break;
        }
    }

    bail!("failed to locate the project directory");
}

fn do_main() -> Result<()> {
    move_to_project_dir()?;

    let args = Cli::parse();
    commands::run_command(args.command)?;

    Ok(())
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

    if let Err(err) = do_main() {
        error!("{}", err);
    }
}
