#![no_std]
use starina::{info, worker::App};

pub struct Main {}

impl App for Main {
    fn init() -> Self {
        info!("Hello from ktest!");
        Main {}
    }
}
