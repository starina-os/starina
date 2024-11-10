use clap::Subcommand;

mod run;

#[derive(Subcommand, Debug)]
pub enum Command {
    Run(run::Args),
}

pub fn run_command(command: Command) {
    match command {
        Command::Run(args) => run::main(args),
    }
}
