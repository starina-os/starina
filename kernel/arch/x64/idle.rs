use core::arch::asm;

use crate::thread::Thread;

pub fn idle() -> ! {
    warn!("idle");
    loop {
        unsafe {
            asm!("sti; hlt; cli");
            Thread::switch(); // FIXME: here should be unreachable
        }
    }
}
