use anyhow::Result;
use clap::Subcommand;

mod build;

#[derive(Subcommand, Debug)]
pub enum Command {
    Build(build::Args),
}

pub async fn run_command(command: Command) -> Result<()> {
    match command {
        Command::Build(args) => build::main(args).await,
    }
}
