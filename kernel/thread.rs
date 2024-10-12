//! Kernel-level thread.
use core::cell::RefMut;

use ftl_types::error::FtlError;
use ftl_types::poll::PollSyscallResult;

use crate::arch;
use crate::channel::Channel;
use crate::cpuvar::current_thread;
use crate::poll::Poll;
use crate::process::kernel_process;
use crate::process::Process;
use crate::refcount::SharedRef;
use crate::scheduler::GLOBAL_SCHEDULER;
use crate::spinlock::SpinLock;
use crate::uaddr::UAddr;

/// The blocked thread state.
#[derive(Debug)]
pub enum Continuation {
    /// Waiting for a message (`channel_recv` or `channel_call` system calls).
    ChannelRecv {
        process: SharedRef<Process>,
        channel: SharedRef<Channel>,
        msgbuffer: UAddr,
    },
    /// Waiting for a poll event (`poll_wait` system call).
    PollWait { poll: SharedRef<Poll> },
}

pub enum ContinuationResult {
    ReturnToUser(Option<isize>),
    StillBlocked,
}

#[derive(Debug)]
enum State {
    Runnable,
    Blocked(Continuation),
    Exited,
}

struct Mutable {
    state: State,
}

pub struct Thread {
    mutable: SpinLock<Mutable>,
    arch: arch::Thread,
    process: SharedRef<Process>,
}

impl Thread {
    pub fn new_idle() -> SharedRef<Thread> {
        SharedRef::new(Thread {
            mutable: SpinLock::new(Mutable {
                state: State::Runnable,
            }),
            arch: arch::Thread::new_idle(),
            process: kernel_process().clone(),
        })
    }

    pub fn spawn_kernel(
        process: SharedRef<Process>,
        pc: usize,
        sp: usize,
        arg: usize,
    ) -> SharedRef<Thread> {
        let thread = SharedRef::new(Thread {
            mutable: SpinLock::new(Mutable {
                state: State::Runnable,
            }),
            arch: arch::Thread::new_kernel(pc, sp, arg),
            process,
        });

        GLOBAL_SCHEDULER.push(thread.clone());
        thread
    }

    pub fn is_runnable(&self) -> bool {
        matches!(self.mutable.lock().state, State::Runnable)
    }

    pub fn process(&self) -> &SharedRef<Process> {
        &self.process
    }

    pub const fn arch(&self) -> &arch::Thread {
        &self.arch
    }

    pub fn set_runnable(&self) {
        let mut mutable = self.mutable.lock();
        mutable.state = State::Runnable;
    }

    pub fn set_exited(&self) {
        let mut mutable = self.mutable.lock();
        mutable.state = State::Exited;
    }

    pub fn run_continuation(this: RefMut<'_, SharedRef<Self>>) -> ContinuationResult {
        let mut mutable = this.mutable.lock();
        let continuation = match &mut mutable.state {
            State::Exited => {
                unreachable!()
            }
            State::Blocked(continuation) => continuation,
            State::Runnable => {
                return ContinuationResult::ReturnToUser(None);
            }
        };

        match continuation {
            Continuation::ChannelRecv {
                process,
                channel,
                msgbuffer,
            } => {
                match channel.recv(*msgbuffer, false, process) {
                    Err(FtlError::WouldBlock) => ContinuationResult::StillBlocked,
                    Ok(msginfo) => {
                        mutable.state = State::Runnable;
                        ContinuationResult::ReturnToUser(Some(msginfo.as_raw()))
                    }
                    Err(e) => {
                        mutable.state = State::Runnable;
                        ContinuationResult::ReturnToUser(Some(e as isize))
                    }
                }
            }
            Continuation::PollWait { poll } => {
                match poll.wait(false) {
                    Err(FtlError::WouldBlock) => ContinuationResult::StillBlocked,
                    Ok((event, handle_id)) => {
                        let ret = PollSyscallResult::new(event, handle_id);
                        mutable.state = State::Runnable;
                        ContinuationResult::ReturnToUser(Some(ret.as_raw()))
                    }
                    Err(e) => {
                        mutable.state = State::Runnable;
                        ContinuationResult::ReturnToUser(Some(e as isize))
                    }
                }
            }
        }
    }

    pub fn block_current(continuation: Continuation) -> ! {
        let thread = current_thread();
        let mut mutable = thread.mutable.lock();
        mutable.state = State::Blocked(continuation);

        drop(mutable);
        drop(thread);

        Self::switch();
    }

    pub fn switch() -> ! {
        switch_thread();
    }

    pub fn push_to_runqueue(this: SharedRef<Thread>) {
        GLOBAL_SCHEDULER.push(this);
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

        // Switch to the new thread's address space.sstatus,a1
        current_thread.process.vmspace().switch();

        // Execute the pending continuation if any.
        let arch_thread: *mut arch::Thread = current_thread.arch() as *const _ as *mut _;
        let result = Thread::run_continuation(current_thread);

        // Can we resume the thread?
        match result {
            ContinuationResult::StillBlocked => {
                warn!("thread is still blocked");
                continue;
            }
            ContinuationResult::ReturnToUser(ret) => {
                arch::return_to_user(arch_thread, ret);
            }
        }
    }
}
