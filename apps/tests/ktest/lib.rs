#![no_std]
use starina::app::App;
use starina::info;
use starina::syscall::thread_yield;

pub struct Main {}

impl App for Main {
    fn init() -> Self {
        info!("Hello from ktest!");
        Main {}
    }

    fn heartbeat(&self) {
        info!("Heartbeat from ktest!");
        thread_yield();
    }
}
