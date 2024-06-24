use alloc::vec::Vec;

use crate::arch;
use crate::cpuvar::current_thread;
use crate::ref_counted::SharedRef;
use crate::spinlock::SpinLock;
use crate::spinlock::SpinLockGuard;
use crate::thread::Thread;

struct Poller {
    thread: SharedRef<Thread>,
}

pub enum PollResult<T> {
    Ready(T),
    Sleep,
}

pub struct PollPoint {
    pollers: SpinLock<Vec<Poller>>,
}

impl PollPoint {
    pub const fn new() -> PollPoint {
        PollPoint {
            pollers: SpinLock::new(Vec::new()),
        }
    }

    pub fn wake(&self) {
        for poller in self.pollers.lock().drain(..) {
            poller.thread.set_runnable();
        }
    }

    pub fn poll_loop<'a, F, T, U>(&self, lock: &'a SpinLock<T>, is_ready: F) -> U
    where
        F: Fn(&mut SpinLockGuard<'a, T>) -> PollResult<U>,
    {
        loop {
            let mut pollers = self.pollers.lock();
            let mut guard = lock.lock();
            match is_ready(&mut guard) {
                PollResult::Ready(ret) => {
                    return ret;
                }
                PollResult::Sleep => {
                    let current_thread = current_thread();
                    pollers.push(Poller {
                        thread: current_thread.clone(),
                    });

                    current_thread.set_blocked();

                    // Release the lock.
                    drop(current_thread);
                    drop(guard);
                    drop(pollers);

                    arch::yield_cpu();
                }
            }
        }
    }
}
