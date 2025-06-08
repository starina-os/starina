use starina::timer::MonotonicTime;

pub fn set_timer_frequency(_freq: u64) {
    // No-op for host
}

pub fn get_monotonic_time() -> MonotonicTime {
    // For host, return a dummy time
    MonotonicTime::from_raw(0)
}

pub fn schedule_timer_interrupt(_expire_time_ns: u64) {
    // No-op for host
}