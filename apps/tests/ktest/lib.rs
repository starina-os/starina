#![no_std]
use starina::app::App;
use starina::info;
use starina::syscall::poll_create;
use starina::syscall::poll_wait;
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

        info!("ktest: poll_create");
        let poll = poll_create().unwrap();
        info!("ktest: poll_wait");
        let ev = poll_wait(poll).unwrap();

        info!("ktest: poll_wait={:?}", ev);
        self.counter += 1;
        thread_yield();
    }
}

// TODO: Remove this.
pub fn app_main() {
    starina::app::app_loop(Main::init());
}
