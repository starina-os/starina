use anyhow::Result;
use clap::Parser;

use crate::make::run_make;

#[derive(Parser, Debug)]
pub struct Args {}

pub fn main(_args: Args) -> Result<()> {
    run_make("starina.elf")?;
    Ok(())
}
