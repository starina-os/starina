use alloc::collections::VecDeque;

use starina_types::error::ErrorCode;

use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;
use crate::thread::Thread;

pub static GLOBAL_SCHEDULER: Scheduler = Scheduler::new();

pub struct Scheduler {
    runqueue: SpinLock<VecDeque<SharedRef<Thread>>>,
}

impl Scheduler {
    pub const fn new() -> Scheduler {
        Scheduler {
            runqueue: SpinLock::new(VecDeque::new()),
        }
    }

    pub fn push(&self, new_thread: SharedRef<Thread>) {
        // SAFETY: This should not panic because we've already reserved the
        //         capacity in `try_reserve`.
        self.runqueue.lock().push_back(new_thread);
    }

    pub fn try_reserve_cap(&self, new_cap: usize) -> Result<(), ErrorCode> {
        let mut runqueue = self.runqueue.lock();
        if let Some(additional) = new_cap.checked_sub(runqueue.capacity()) {
            runqueue
                .try_reserve(additional)
                .map_err(|_| ErrorCode::OutOfMemory)?;
        }

        Ok(())
    }

    pub fn schedule(
        &self,
        thread_to_enqueue: Option<SharedRef<Thread>>,
    ) -> Option<SharedRef<Thread>> {
        let mut runqueue = self.runqueue.lock();

        if let Some(thread) = thread_to_enqueue {
            runqueue.push_back(thread);
        }

        runqueue.pop_front()
    }
}
