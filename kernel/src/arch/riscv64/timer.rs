use core::arch::asm;

pub fn read_timer() -> u64 {
    let time: u64;
    unsafe {
        asm!("rdtime {}", out(reg) time);
    }

    time
}

pub fn set_timer(ticks: u64) {
    unsafe {
        asm!("csrw stimecmp, {}", in(reg) ticks);
    }
}
