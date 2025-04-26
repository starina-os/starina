use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use starina_types::error::ErrorCode;
use starina_types::syscall::RetVal;

use crate::arch;
use crate::arch::VmSpace;
use crate::poll::Poll;
use crate::process::KERNEL_PROCESS;
use crate::process::Process;
use crate::refcount::SharedRef;
use crate::scheduler::GLOBAL_SCHEDULER;
use crate::spinlock::SpinLock;
use crate::syscall::SyscallResult;
use crate::vcpu::VCpu;

static NUM_THREADS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
pub enum ThreadState {
    Runnable(Option<RetVal>),
    BlockedByPoll(SharedRef<Poll>),
    RunVCpu(SharedRef<VCpu>),
    InVCpu(SharedRef<VCpu>),
    Exited,
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
        debug_assert!(matches!(
            mutable.state,
            ThreadState::Runnable(_) | ThreadState::BlockedByPoll(_)
        ));

        let was_blocked = !matches!(mutable.state, ThreadState::Runnable(_));

        // Update the thread's state.
        mutable.state = new_state;

        // If the thread is now runnable, push it to the scheduler.
        if was_blocked && matches!(mutable.state, ThreadState::Runnable(_)) {
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
        let (mut current_thread, is_idle, is_runnable) = {
            // Borrow the cpvuar inside a brace not to forget to drop it.
            let cpuvar = arch::get_cpuvar();

            let current_thread = cpuvar.current_thread.borrow_mut();
            let is_idle = SharedRef::ptr_eq(&*current_thread, &cpuvar.idle_thread);
            let is_runnable = matches!(
                current_thread.mutable.lock().state,
                ThreadState::Runnable(_)
            );
            (current_thread, is_idle, is_runnable)
        };

        let next = if is_runnable && !is_idle {
            // If the current thread is still runnable, prioritize it because
            // it might be sending multiple messages in a row.
            current_thread.clone()
        } else if let Some(next) = GLOBAL_SCHEDULER.schedule() {
            // Get the next thread to run. If the runqueue is empty, run the
            // idle thread.
            next
        } else {
            drop(current_thread);
            arch::idle();
        };

        // Make the next thread the current thread.
        *current_thread = next;

        // Try unblocking the next thread.
        let arch_thread = {
            let mut mutable = current_thread.mutable.lock();
            let retval = match &mutable.state {
                ThreadState::BlockedByPoll(poll) => {
                    match poll.try_wait(&current_thread) {
                        SyscallResult::Done(result) => {
                            // We've got an event. Resume the thread with a return
                            // value.
                            Some(result.into())
                        }
                        SyscallResult::Err(err) => {
                            // The poll is no longer valid. Return the error as a
                            // syscall result.
                            Some(err.into())
                        }
                        SyscallResult::Block(new_state) => {
                            // The thread is still blocked. We'll retry when the
                            // poll wakes us up again...
                            mutable.state = new_state;
                            continue 'next_thread;
                        }
                    }
                }
                ThreadState::RunVCpu(vcpu) => {
                    // Keep at least one reference to vcpu in the state to keep alive.
                    let vcpu_ptr = unsafe { vcpu.arch_vcpu_ptr() };
                    mutable.state = ThreadState::InVCpu(vcpu.clone());
                    drop(mutable);
                    drop(current_thread);
                    arch::vcpu_entry(vcpu_ptr);
                }
                ThreadState::InVCpu(vcpu) => {
                    mutable.state = ThreadState::Runnable(None);
                    // The return value from vcpu_run syscall.
                    Some(RetVal::new(0))
                }
                ThreadState::Exited => {
                    continue 'next_thread;
                }
                ThreadState::Runnable(retval) => *retval,
            };

            // The thread is runnable. Get ready to restore the thread's context.
            unsafe {
                let arch = mutable.arch_thread_ptr();

                // If we're returning from a system call, set the return value.
                if let Some(retval) = retval {
                    (*arch).set_retval(retval);
                }

                arch
            }
        };

        // Switch to the next thread's address space.
        current_thread.process().vmspace().switch();

        // Execute the pending continuation if any.
        drop(current_thread);
        arch::user_entry(arch_thread);
    }
}
