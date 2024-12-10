use core::arch::{ global_asm};

global_asm!(include_str!("start.S"));

mod syscall;

pub use syscall::syscall;

