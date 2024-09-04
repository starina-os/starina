use alloc::collections::VecDeque;

use crate::ref_counted::SharedRef;
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
        self.runqueue.lock().push_back(new_thread);
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
