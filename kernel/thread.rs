use crate::arch;
use crate::poll::Poll;
use crate::process::KERNEL_PROCESS;
use crate::process::Process;
use crate::refcount::SharedRef;
use crate::scheduler::GLOBAL_SCHEDULER;
use crate::spinlock::SpinLock;
use crate::syscall::RetVal;

pub enum ThreadState {
    Runnable,
    ResumeWith(RetVal),
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
    pub fn new_idle() -> SharedRef<Thread> {
        SharedRef::new(Thread {
            mutable: SpinLock::new(Mutable {
                state: ThreadState::Runnable,
                arch: arch::Thread::new_idle(),
            }),
            process: KERNEL_PROCESS.clone(),
        })
    }

    pub fn new_inkernel(pc: usize, arg: usize) -> SharedRef<Thread> {
        let thread = SharedRef::new(Thread {
            mutable: SpinLock::new(Mutable {
                state: ThreadState::Runnable, // TODO: Mark as blocked by default.
                arch: arch::Thread::new_inkernel(pc, arg),
            }),
            process: KERNEL_PROCESS.clone(),
        });

        GLOBAL_SCHEDULER.push(thread.clone());
        thread
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
        debug_assert_ne!(
            core::mem::discriminant(&mutable.state),
            core::mem::discriminant(&new_state)
        );

        mutable.state = new_state;
        GLOBAL_SCHEDULER.push(self.clone());
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

        // Continue the thread's work depending on its state.
        let arch_thread = {
            let mut next_mutable = next.mutable.lock();
            match &next_mutable.state {
                ThreadState::Runnable => {
                    // Nothing to do. Just continue running the thread in the userspace.
                    unsafe { next_mutable.arch_thread_ptr() }
                }
                ThreadState::ResumeWith(retval) => unsafe {
                    let arch = next_mutable.arch_thread_ptr();
                    (*arch).set_retval(*retval);
                    arch
                },
                ThreadState::BlockedByPoll(poll) => {
                    if let Some(result) = poll.try_wait() {
                        next_mutable.state = ThreadState::ResumeWith(result.into());
                        GLOBAL_SCHEDULER.push(next.clone());
                    }

                    continue 'next_thread;
                }
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
