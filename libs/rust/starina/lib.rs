#![no_std]

pub trait Worker {
    type Context;
    fn init() -> Self;
}
