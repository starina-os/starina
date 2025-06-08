use starina::timer::MonotonicTime;
use crate::spinlock::SpinLock;
use super::csr::{read_time, write_stimecmp};

static TIMER_FREQUENCY: SpinLock<Option<u64>> = SpinLock::new(None);

pub fn set_timer_frequency(freq: u64) {
    let mut timer_freq = TIMER_FREQUENCY.lock();
    *timer_freq = Some(freq);
}

pub fn get_monotonic_time() -> MonotonicTime {
    let timer_freq = TIMER_FREQUENCY.lock();
    let freq = timer_freq.unwrap_or(10_000_000); // Default to 10 MHz if not set (QEMU aclint-mtimer)
    
    let time_cycles = unsafe { read_time() };
    let time_ns = (time_cycles * 1_000_000_000) / freq;
    MonotonicTime::from_raw(time_ns)
}

pub fn schedule_timer_interrupt(expire_time_ns: u64) {
    let timer_freq = TIMER_FREQUENCY.lock();
    let freq = timer_freq.unwrap_or(10_000_000); // Default to 10 MHz if not set (QEMU aclint-mtimer)
    
    let expire_cycles = (expire_time_ns * freq) / 1_000_000_000;
    unsafe {
        write_stimecmp(expire_cycles);
    }
}