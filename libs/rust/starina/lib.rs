#![no_std]
#![cfg_attr(test, feature(test))]

pub trait Worker {
    type Context: 'static;
    fn init() -> Self;
    fn call(&self, context: &Self::Context) {}
}
