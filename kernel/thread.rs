use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use starina::error::ErrorCode;

use crate::arch;
use crate::poll::Poll;
use crate::process::KERNEL_PROCESS;
use crate::process::Process;
use crate::refcount::SharedRef;
use crate::scheduler::GLOBAL_SCHEDULER;
use crate::spinlock::SpinLock;
use crate::syscall::RetVal;

static NUM_THREADS: AtomicUsize = AtomicUsize::new(0);

pub enum ThreadState {
    Runnable(Option<RetVal>),
    BlockedByPoll(SharedRef<Poll>),
}

struct Mutable {
    state: ThreadState,
    arch: arch::Thread,
}

impl Mutable {
    unsafe fn arch_thread_ptr(&self) -> *mut arch::Thread {
        &raw const self.arch as *mut _
    }
}

pub struct Thread {
    mutable: SpinLock<Mutable>,
    process: SharedRef<Process>,
}

impl Thread {
    pub fn new_idle() -> Result<SharedRef<Thread>, ErrorCode> {
        SharedRef::new(Thread {
            mutable: SpinLock::new(Mutable {
                state: ThreadState::Runnable(None),
                arch: arch::Thread::new_idle(),
            }),
            process: KERNEL_PROCESS.clone(),
        })
    }

    pub fn new_inkernel(pc: usize, arg: usize) -> Result<SharedRef<Thread>, ErrorCode> {
        let thread = SharedRef::new(Thread {
            mutable: SpinLock::new(Mutable {
                state: ThreadState::Runnable(None), // TODO: Mark as blocked by default.
                arch: arch::Thread::new_inkernel(pc, arg),
            }),
            process: KERNEL_PROCESS.clone(),
        })?;

        let old_num_threads = NUM_THREADS.fetch_add(1, Ordering::Relaxed);
        GLOBAL_SCHEDULER.try_reserve_cap(old_num_threads + 1)?;

        GLOBAL_SCHEDULER.push(thread.clone());
        Ok(thread)
    }

    pub unsafe fn arch_thread_ptr(&self) -> *mut arch::Thread {
        let mutable = self.mutable.lock();
        unsafe { mutable.arch_thread_ptr() }
    }

    pub fn process(&self) -> &SharedRef<Process> {
        &self.process
    }

    pub fn wake(self: &SharedRef<Self>) {
        GLOBAL_SCHEDULER.push(self.clone());
    }

    pub fn set_state(self: &SharedRef<Thread>, new_state: ThreadState) {
        let mut mutable = self.mutable.lock();

        // We should never change the state to the same state.
        debug_assert_ne!(
            core::mem::discriminant(&mutable.state),
            core::mem::discriminant(&new_state)
        );

        // Update the thread's state.
        mutable.state = new_state;

        // If the thread is now runnable, push it to the scheduler.
        if matches!(mutable.state, ThreadState::Runnable(_)) {
            GLOBAL_SCHEDULER.push(self.clone());
        }
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        NUM_THREADS.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Switches to the thread execution: save the current thread, picks the next
/// thread to run, and restores the next thread's context.
pub fn switch_thread() -> ! {
    'next_thread: loop {
        let (mut current_thread, in_idle) = {
            // Borrow the cpvuar inside a brace not to forget to drop it.
            let cpuvar = arch::get_cpuvar();

            let current_thread = cpuvar.current_thread.borrow_mut();
            let in_idle = SharedRef::ptr_eq(&*current_thread, &cpuvar.idle_thread);
            (current_thread, in_idle)
        };

        // Preemptive scheduling: push the current thread back to the
        // runqueue if it's still runnable.
        let thread_to_enqueue = if in_idle {
            None
        } else {
            Some(current_thread.clone())
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

        // Try unblocking the next thread.
        let arch_thread = {
            let mut next_mutable = next.mutable.lock();
            match &next_mutable.state {
                ThreadState::BlockedByPoll(poll) => {
                    if let Some(result) = poll.try_wait() {
                        // We've got an event. Make the thread runnable again
                        // with the system call's return value.
                        next_mutable.state = ThreadState::Runnable(Some(result.into()));
                        GLOBAL_SCHEDULER.push(next.clone());
                    } else {
                        // The thread is still blocked. We'll retry when the
                        // poll wakes us up again...
                    }

                    continue 'next_thread;
                }
                ThreadState::Runnable(retval) => unsafe {
                    let arch = next_mutable.arch_thread_ptr();

                    // If we're returning from a system call, set the return value.
                    if let Some(retval) = retval {
                        (*arch).set_retval(*retval);
                    }

                    arch
                },
            }
        };

        // Make the next thread the current thread.
        *current_thread = next;

        // TODO: Switch to the new thread's address space.sstatus,a1
        // current_thread.process.vmspace().switch();

        // Execute the pending continuation if any.
        drop(current_thread);
        arch::enter_userland(arch_thread);
    }
}
