#![no_std]

pub trait Worker {
    fn init() -> Self;
}
