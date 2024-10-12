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

        arch::return_to_user();
    }

    pub fn switch() -> ! {
        arch::return_to_user();
    }

    pub fn push_to_runqueue(this: SharedRef<Thread>) {
        GLOBAL_SCHEDULER.push(this);
    }
}
