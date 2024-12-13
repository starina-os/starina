use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
}

pub async fn main(args: Args) -> Result<()> {
    Ok(())
}
