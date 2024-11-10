use anyhow::Result;
use clap::Subcommand;

mod build;
mod run;
mod scaffold;

#[derive(Subcommand, Debug)]
pub enum Command {
    Build(build::Args),
    Run(run::Args),
    Scaffold(scaffold::Args),
}

pub fn run_command(command: Command) -> Result<()> {
    match command {
        Command::Build(args) => build::main(args),
        Command::Run(args) => run::main(args),
        Command::Scaffold(args) => scaffold::main(args),
    }
}
