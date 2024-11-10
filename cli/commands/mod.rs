use anyhow::Result;
use clap::Subcommand;

mod build;
mod run;
mod scaffold;

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Build the OS.
    Build(build::Args),
    /// Build and run the OS.
    Run(run::Args),
    /// Generate boilterplates.
    Scaffold(scaffold::Args),
}

pub fn run_command(command: Command) -> Result<()> {
    match command {
        Command::Build(args) => build::main(args),
        Command::Run(args) => run::main(args),
        Command::Scaffold(args) => scaffold::main(args),
    }
}
