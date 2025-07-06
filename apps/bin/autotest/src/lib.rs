#![no_std]

mod channel;

use starina::prelude::*;
use starina::spec::AppSpec;

use crate::channel::test_channel;

pub const SPEC: AppSpec = AppSpec {
    name: "autotest",
    env: &[],
    exports: &[],
    main,
};

fn main(_env_json: &[u8]) {
    info!("Starting tests...");
    test_channel();
    info!("Passed all tests!");
}
