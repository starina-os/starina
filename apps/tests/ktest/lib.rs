#![no_std]
use starina::Worker;

pub struct App;

impl Worker for App {
    type Context = ();

    fn init() -> Self {
        App
    }
}
