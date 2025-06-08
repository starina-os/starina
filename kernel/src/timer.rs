use alloc::vec::Vec;
use core::fmt;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;

use starina::error::ErrorCode;
use starina::poll::Readiness;
use starina_types::timer::MonotonicTime;

use crate::arch;
use crate::handle::Handleable;
use crate::poll::ListenerSet;
use crate::poll::Poll;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;

#[derive(Debug, Clone)]
enum State {
    NotSet,
    Expired,
    Pending(u64 /* ticks */),
}

struct Mutable {
    state: State,
    listeners: ListenerSet,
}

pub struct Timer {
    mutable: SpinLock<Mutable>,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            mutable: SpinLock::new(Mutable {
                state: State::NotSet,
                listeners: ListenerSet::new(),
            }),
        }
    }

    pub fn set_timeout(self: &SharedRef<Self>, duration_ns: u64) -> Result<(), ErrorCode> {
        let now_ticks = arch::read_timer();
        let freq = TIMER_FREQ.load(Ordering::Relaxed);
        let duration_ticks = ns_to_ticks(duration_ns, freq);

        // Guarantee that is_tick_before and is_timer_expired work correctly.
        if duration_ticks > u64::MAX / 2 {
            return Err(ErrorCode::InvalidArg);
        }

        let expires_at = now_ticks.wrapping_add(duration_ticks);

        let mut global_timer = GLOBAL_TIMER.lock();

        let mut mutable = self.mutable.lock();
        mutable.state = State::Pending(expires_at);
        drop(mutable);

        global_timer.actives.push(self.clone());
        reschedule_timer(&global_timer);
        Ok(())
    }
}

impl Handleable for Timer {
    fn close(&self) {
        // TODO: Should we do anything here?
    }

    fn add_listener(&self, listener: crate::poll::Listener) -> Result<(), ErrorCode> {
        let mut mutable = self.mutable.lock();
        mutable.listeners.add_listener(listener)
    }

    fn remove_listener(&self, poll: &Poll) -> Result<(), ErrorCode> {
        let mut mutable = self.mutable.lock();
        mutable.listeners.remove_listener(poll);
        Ok(())
    }

    fn readiness(&self) -> Result<Readiness, ErrorCode> {
        let mut readiness = Readiness::new();

        let mutable = self.mutable.lock();
        if matches!(mutable.state, State::Expired) {
            readiness |= Readiness::READABLE;
        }

        Ok(readiness)
    }
}

impl fmt::Debug for Timer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Timer")
    }
}

pub static TIMER_FREQ: AtomicU64 = AtomicU64::new(0);

struct GlobalTimer {
    actives: Vec<SharedRef<Timer>>,
}

impl GlobalTimer {
    pub const fn new() -> Self {
        Self {
            actives: Vec::new(),
        }
    }
}

static GLOBAL_TIMER: SpinLock<GlobalTimer> = SpinLock::new(GlobalTimer::new());

fn ns_to_ticks(ns: u64, freq: u64) -> u64 {
    (ns * freq) / 1_000_000_000
}

fn ticks_to_monotonic_time(ticks: u64, freq: u64) -> MonotonicTime {
    // Convert ticks to nanoseconds: nanos = ticks * 1_000_000_000 / freq
    let nanos = ticks * 1_000_000_000 / freq;
    MonotonicTime::from_nanos(nanos)
}

/// Compare two tick values considering potential wrapping.
///
/// Returns true if `a` is before `b` in circular time. This works correctly
/// even when the tick counter wraps around.
fn is_tick_before(a: u64, b: u64) -> bool {
    // If the difference is less than the maximum timer duration, a is before b.
    b.wrapping_sub(a) < (u64::MAX / 2)
}

/// Returns true if the timer has expired (now >= expires_at) considering wrapping.
fn is_timer_expired(now: u64, expires_at: u64) -> bool {
    // Timer is expired if now is at or after expires_at
    // This means expires_at is before or equal to now
    let diff = now.wrapping_sub(expires_at);
    diff < (u64::MAX / 2)
}

pub fn init(freq: u64) {
    debug_assert_ne!(freq, 0);

    TIMER_FREQ.store(freq, Ordering::Relaxed);
    info!("timer initialized with frequency: {} Hz", freq);
}

/// Get the current monotonic time since kernel boot.
pub fn now() -> MonotonicTime {
    let freq = TIMER_FREQ.load(Ordering::Relaxed);
    debug_assert_ne!(freq, 0, "timer not initialized");

    let ticks = arch::read_timer();
    ticks_to_monotonic_time(ticks, freq)
}

// Reschedule for the next earliest timer.
fn reschedule_timer(global_timer: &GlobalTimer) {
    let mut earliest = None;
    for timer in &global_timer.actives {
        let mutable = timer.mutable.lock();
        if let State::Pending(expires_ticks) = mutable.state
            && (earliest.is_none() || is_tick_before(expires_ticks, earliest.unwrap())) {
                earliest = Some(expires_ticks);
            }
    }

    if let Some(timeout) = earliest {
        arch::set_timer(timeout);
    }
}

pub fn handle_timer_interrupt() {
    let now_ticks = arch::read_timer();
    let mut global_timer = GLOBAL_TIMER.lock();

    // Check all timers and remove expired ones.
    let mut new_actives = Vec::new();
    for timer in &global_timer.actives {
        let mut mutable = timer.mutable.lock();
        match mutable.state {
            State::Pending(expires_at) if is_timer_expired(now_ticks, expires_at) => {
                // The timer has expired, notify the listeners.
                mutable.state = State::Expired;
                mutable.listeners.notify_all(Readiness::READABLE);
            }
            State::Pending(_) => {
                // The timer is still pending, keep it in the active list.
                new_actives.push(timer.clone());
            }
            _ => {
                unreachable!("timer is active but not pending");
            }
        }
    }

    global_timer.actives = new_actives;
    reschedule_timer(&global_timer);
}
