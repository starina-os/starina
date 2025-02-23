use crate::arch;
use crate::refcount::SharedRef;
use crate::scheduler::GLOBAL_SCHEDULER;
use crate::spinlock::SpinLock;

enum State {
    Blocked,
    Runnable,
}

struct Mutable {
    state: State,
}

pub struct Thread {
    mutable: SpinLock<Mutable>,
    arch: arch::Thread,
}

impl Thread {
    pub fn new_idle() -> SharedRef<Thread> {
        SharedRef::new(Thread {
            mutable: SpinLock::new(Mutable {
                state: State::Runnable,
            }),
            arch: arch::Thread::new_idle(),
        })
    }

    pub fn is_runnable(&self) -> bool {
        matches!(self.mutable.lock().state, State::Runnable)
    }

    pub const fn arch(&self) -> &arch::Thread {
        &self.arch
    }
}

/// Switches to the thread execution: save the current thread, picks the next
/// thread to run, and restores the next thread's context.
pub fn switch_thread() -> ! {
    loop {
        let (mut current_thread, in_idle) = {
            // Borrow the cpvuar inside a brace not to forget to drop it.
            let cpuvar = arch::get_cpuvar();

            let current_thread = cpuvar.current_thread.borrow_mut();
            let in_idle = SharedRef::ptr_eq(&*current_thread, &cpuvar.idle_thread);
            (current_thread, in_idle)
        };

        // Preemptive scheduling: push the current thread back to the
        // runqueue if it's still runnable.
        let thread_to_enqueue = if current_thread.is_runnable() && !in_idle {
            Some(current_thread.clone())
        } else {
            None
        };

        // Get the next thread to run. If the runqueue is empty, run the
        // idle thread.
        let next = match GLOBAL_SCHEDULER.schedule(thread_to_enqueue) {
            Some(next) => next,
            None => {
                drop(current_thread);
                arch::idle();
            }
        };

        // Make the next thread the current thread.
        *current_thread = next;

        // TODO: Switch to the new thread's address space.sstatus,a1
        // current_thread.process.vmspace().switch();

        // Execute the pending continuation if any.
        let arch_thread: *mut arch::Thread = current_thread.arch() as *const _ as *mut _;

        panic!("switching to {:#x?}", arch_thread);
    }
}
