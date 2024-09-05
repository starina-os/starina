use core::num::NonZeroIsize;
use core::sync::atomic::AtomicIsize;
use core::sync::atomic::Ordering;

use ftl_types::error::FtlError;
use ftl_types::poll::PollSyscallResult;

use crate::arch;
use crate::channel::Channel;
use crate::cpuvar::current_thread;
use crate::poll::Poll;
use crate::process::kernel_process;
use crate::process::Process;
use crate::ref_counted::SharedRef;
use crate::scheduler::GLOBAL_SCHEDULER;
use crate::spinlock::SpinLock;
use crate::uaddr::UAddr;
use crate::vmspace::VmSpace;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThreadId(NonZeroIsize);

impl ThreadId {
    pub fn new_idle() -> ThreadId {
        // SAFETY: -1 is a valid NonZeroIsize value.
        let value = unsafe { NonZeroIsize::new_unchecked(-1) };
        ThreadId(value)
    }

    pub fn alloc() -> ThreadId {
        static NEXT_ID: AtomicIsize = AtomicIsize::new(1);

        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        // SAFETY: fetch_add may wrap around, but it should be fine unless you
        //         run the system for soooooo long years.
        let value = unsafe { NonZeroIsize::new_unchecked(id) };
        ThreadId(value)
    }

    pub fn as_isize(&self) -> isize {
        self.0.get()
    }
}

#[derive(Debug)]
pub enum Continuation {
    ChannelRecv {
        channel: SharedRef<Channel>,
        msgbuffer: UAddr,
    },
    PollWait {
        poll: SharedRef<Poll>,
    },
}

pub enum ContinuationResult {
    ReturnToUser(Option<isize>),
    StillBlocked,
}

#[derive(Debug)]
enum State {
    Runnable,
    Blocked(Continuation),
}

struct Mutable {
    state: State,
}

pub struct Thread {
    id: ThreadId,
    mutable: SpinLock<Mutable>,
    arch: arch::Thread,
    process: SharedRef<Process>,
}

impl Thread {
    pub fn new_idle() -> SharedRef<Thread> {
        SharedRef::new(Thread {
            id: ThreadId::new_idle(),
            mutable: SpinLock::new(Mutable {
                state: State::Runnable,
            }),
            arch: arch::Thread::new_idle(),
            process: kernel_process().clone(),
        })
    }

    pub fn spawn_kernel(
        process: SharedRef<Process>,
        vmspace: SharedRef<VmSpace>,
        pc: usize,
        sp: usize,
        arg: usize,
    ) -> SharedRef<Thread> {
        let thread = SharedRef::new(Thread {
            id: ThreadId::alloc(),
            mutable: SpinLock::new(Mutable {
                state: State::Runnable,
            }),
            arch: arch::Thread::new_kernel(vmspace, pc as usize, sp, arg),
            process,
        });

        GLOBAL_SCHEDULER.push(thread.clone());
        thread
    }

    pub fn id(&self) -> ThreadId {
        self.id
    }

    pub fn is_idle_thread(&self) -> bool {
        self.id.as_isize() == -1
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

    pub fn run_continuation(&self) -> ContinuationResult {
        let mut mutable = self.mutable.lock();
        let continuation = match &mut mutable.state {
            State::Blocked(continuation) => continuation,
            State::Runnable => {
                return ContinuationResult::ReturnToUser(None);
            }
        };

        match continuation {
            Continuation::ChannelRecv { channel, msgbuffer } => {
                match channel.recv(*msgbuffer, false) {
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
