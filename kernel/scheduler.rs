use alloc::collections::VecDeque;

use crate::arch::cpuvar;
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

    pub fn yield_cpu(&self) {
        let next = {
            let cpuvar = cpuvar();
            let mut current_thread = cpuvar.current_thread.borrow_mut();
            let mut runqueue = self.runqueue.lock();

            // Preemptive scheduling: push the current thread back to the
            // runqueue if it's still runnable.
            if current_thread.is_runnable() && !current_thread.is_idle_thread() {
                runqueue.push_back(current_thread.clone());
            }

            // Get the next thread to run. If the runqueue is empty, run the
            // idle thread.
            let next = runqueue
                .pop_front()
                .unwrap_or_else(|| cpuvar.idle_thread.clone());

            *current_thread = next.clone();
            next
        };

        next.resume();
    }
}
