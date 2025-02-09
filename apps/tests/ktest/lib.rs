#![no_std]
use starina::Worker;

pub struct App;

impl Worker for App {
    fn init() -> Self {
        App
    }
}
