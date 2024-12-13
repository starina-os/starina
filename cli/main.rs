use clap::Parser;

mod commands;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: commands::Command,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    if let Err(err) = commands::run_command(args.command).await {
        panic!("cli: error: {}", err);
    }
}
