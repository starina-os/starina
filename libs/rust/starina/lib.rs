#![no_std]

pub trait Worker {
    type Context: 'static;
    fn init() -> Self;
    fn call(&self, context: &Self::Context) {}
}
