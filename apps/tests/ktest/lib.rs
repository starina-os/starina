#![no_std]
use starina::{info, app::App};

pub struct Main {}

impl App for Main {
    fn init() -> Self {
        info!("Hello from ktest!");
        Main {}
    }
}
