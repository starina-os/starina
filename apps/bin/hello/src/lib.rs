#![no_std]

use starina::environ::Environ;
use starina::prelude::*;
use starina::spec::AppSpec;

pub const SPEC: AppSpec = AppSpec {
    name: "hello",
    env: &[],
    exports: &[],
    main,
};

fn main(_environ: Environ) {
    info!("Hello, World!");
}
