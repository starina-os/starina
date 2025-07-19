#![no_std]

mod channel;

use starina::environ::Environ;
use starina::prelude::*;
use starina::spec::AppSpec;

use crate::channel::test_channel;

pub const SPEC: AppSpec = AppSpec {
    name: "autotest",
    env: &[],
    exports: &[],
    main,
};

fn main(_environ: Environ) {
    info!("Starting tests...");
    test_channel();
    info!("Passed all tests!");
}
