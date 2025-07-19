#![no_std]

use starina::prelude::*;
use starina::spec::AppSpec;

pub const SPEC: AppSpec = AppSpec {
    name: "hello",
    env: &[],
    exports: &[],
    main,
};

fn main(_env_json: &[u8]) {
    info!("Hello, World!");
}
