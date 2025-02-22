#![no_std]
use starina::{info, worker::Worker};

pub struct App {}

impl Worker for App {
    fn init() -> Self {
        info!("Hello from ktest!");
        App {}
    }
}
