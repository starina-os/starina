use alloc::collections::VecDeque;

use crate::cpuvar::current_thread;
use crate::refcount::SharedRef;
use crate::thread::Thread;

pub struct WaitQueue {
    queue: VecDeque<SharedRef<Thread>>,
}

impl WaitQueue {
    pub const fn new() -> WaitQueue {
        WaitQueue {
            queue: VecDeque::new(),
        }
    }

    pub fn listen(&mut self) {
        let thread = current_thread();
        self.queue.push_back(thread.clone());
    }

    pub fn wake_all(&mut self) {
        for thread in self.queue.drain(..) {
            Thread::push_to_runqueue(thread);
        }
    }
}
