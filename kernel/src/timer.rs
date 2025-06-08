use core::fmt;

use starina::timer::MonotonicTime;
use starina::error::ErrorCode;
use starina::poll::Readiness;

use crate::handle::Handleable;
use crate::poll::ListenerSet;
use crate::poll::Poll;
use crate::spinlock::SpinLock;

struct Mutable {
    expires_at: Option<MonotonicTime>,
    listeners: ListenerSet,
    expired: bool,
}

pub struct Timer {
    mutable: SpinLock<Mutable>,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            mutable: SpinLock::new(Mutable { 
                expires_at: None,
                listeners: ListenerSet::new(),
                expired: false,
            }),
        }
    }

    pub fn set_timeout(self: &crate::refcount::SharedRef<Self>, timeout_ns: u64) -> Result<(), ErrorCode> {
        let mut mutable = self.mutable.lock();
        let current_time = crate::arch::get_monotonic_time();
        mutable.expires_at = Some(MonotonicTime::from_raw(current_time.as_nanos() + timeout_ns));
        mutable.expired = false;
        
        let expires_at_ns = mutable.expires_at.unwrap().as_nanos();
        crate::arch::schedule_timer_interrupt(expires_at_ns);
        
        register_timer(self.clone());
        Ok(())
    }

    pub fn check_expired(&self) -> bool {
        let mut mutable = self.mutable.lock();
        if let Some(expires_at) = mutable.expires_at {
            let current_time = crate::arch::get_monotonic_time();
            if current_time >= expires_at && !mutable.expired {
                mutable.expired = true;
                mutable.listeners.notify_all(Readiness::READABLE);
                return true;
            }
        }
        mutable.expired
    }
}

impl Handleable for Timer {
    fn close(&self) {
        let mut mutable = self.mutable.lock();
        mutable.expires_at = None;
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
        if self.check_expired() {
            Ok(Readiness::READABLE)
        } else {
            Ok(Readiness::new())
        }
    }
}

impl fmt::Debug for Timer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Timer")
    }
}

static ACTIVE_TIMERS: SpinLock<alloc::vec::Vec<crate::refcount::SharedRef<Timer>>> = SpinLock::new(alloc::vec::Vec::new());

pub fn register_timer(timer: crate::refcount::SharedRef<Timer>) {
    let mut active_timers = ACTIVE_TIMERS.lock();
    active_timers.push(timer);
}

pub fn handle_timer_interrupt() {
    let mut active_timers = ACTIVE_TIMERS.lock();
    active_timers.retain(|timer| {
        !timer.check_expired()
    });
}
