use alloc::vec::Vec;

use crate::arch;
use crate::cpuvar::current_thread;
use crate::ref_counted::SharedRef;
use crate::spinlock::SpinLock;
use crate::spinlock::SpinLockGuard;
use crate::thread::Thread;

pub enum SleepCallbackResult<T> {
    Ready(T),
    Sleep,
}

pub struct SleepPoint {
    pollers: SpinLock<Vec<SharedRef<Thread>>>,
}

impl SleepPoint {
    pub const fn new() -> SleepPoint {
        SleepPoint {
            pollers: SpinLock::new(Vec::new()),
        }
    }

    pub fn wake_all(&self) {
        for waiter in self.pollers.lock().drain(..) {
            waiter.set_runnable();
        }
    }

    pub fn sleep_loop<'a, F, T, U>(&self, lock: &'a SpinLock<T>, is_ready: F) -> U
    where
        F: Fn(&mut SpinLockGuard<'a, T>) -> SleepCallbackResult<U>,
    {
        loop {
            let mut waiters = self.pollers.lock();
            let mut guard = lock.lock();
            match is_ready(&mut guard) {
                SleepCallbackResult::Ready(ret) => {
                    return ret;
                }
                SleepCallbackResult::Sleep => {
                    let current_thread = current_thread();
                    waiters.push(current_thread.clone());

                    current_thread.set_blocked();

                    // Release the lock.
                    drop(current_thread);
                    drop(guard);
                    drop(waiters);

                    arch::yield_cpu();
                }
            }
        }
    }
}
