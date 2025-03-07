#![no_std]
use starina::app::App;
use starina::channel::userspace::message::Ping;
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

    fn on_ping(&self, ping: Ping) {
        info!("ktest: ping={}", ping.value());
    }
}

// TODO: Remove this.
pub fn app_main() {
    starina::app::app_loop(Main::init());
}
