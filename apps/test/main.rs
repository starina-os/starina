#![no_std]
#![no_main]

mod channel_test;
mod helpers;

use helpers::Context;
use starina_api::environ::Environ;
use starina_api::prelude::*;

starina_api::autogen!();

struct TestCase {
    name: &'static str,
    test: fn(&mut Context),
}

macro_rules! testcase {
    ($test:expr) => {
        TestCase {
            name: stringify!($test),
            test: $test,
        }
    };
}

const TESTS: &[TestCase] = &[
    testcase!(channel_test::test_channel_call),
    testcase!(channel_test::test_error_reply),
];

#[no_mangle]
pub fn main(mut env: Environ) {
    let echo = env.take_channel("dep:echo").unwrap();
    let mut ctx = Context { echo };

    info!("Running integration tests");
    for TestCase { name, test } in TESTS {
        info!("Running test: {}", name);
        test(&mut ctx);
    }

    info!("all tests {} passed", TESTS.len());
}
