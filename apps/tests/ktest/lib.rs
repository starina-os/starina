#![no_std]
use starina::app::App;
use starina::info;
use starina::syscall::thread_yield;

pub struct Main {
    counter: usize,
}

impl App for Main {
    fn init() -> Self {
        info!("Hello from ktest!");
        Main { counter: 0 }
    }

    fn tick(&mut self) {
        info!("ktest: counter={}", self.counter);
        self.counter += 1;
        thread_yield();
    }
}
