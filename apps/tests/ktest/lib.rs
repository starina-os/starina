#![no_std]
use starina::app::App;
use starina::info;

pub struct Main {}

impl App for Main {
    fn init() -> Self {
        info!("Hello from ktest!");
        Main {}
    }
}
