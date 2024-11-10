use std::os::unix::process::CommandExt;
use std::process::Command;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {}

pub fn main(args: Args) {
    let err = Command::new("make").args(["run"]).exec();
    panic!("failed to exec make: {}", err);
}
