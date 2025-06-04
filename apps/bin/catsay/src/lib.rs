#![no_std]

mod catsay;

use serde::Deserialize;
use starina::spec::AppSpec;
use starina::spec::ExportItem;

pub const SPEC: AppSpec = AppSpec {
    name: "catsay",
    env: &[],
    exports: &[ExportItem::Service { service: "catsay" }],
    main,
};

#[derive(Debug, Deserialize)]
struct Env {}

fn main(env_json: &[u8]) {
    let _env: Env = serde_json::from_slice(env_json).unwrap();
    catsay::catsay("I'm a teapot!");
}
