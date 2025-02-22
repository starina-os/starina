#![no_std]
use starina::worker::Worker;

pub struct App {}

impl Worker for App {
    fn init() -> Self {
        App {}
    }
}
