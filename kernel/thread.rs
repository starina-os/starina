use core::num::NonZeroIsize;
use core::sync::atomic::AtomicIsize;
use core::sync::atomic::Ordering;

use crate::arch;
use crate::process::kernel_process;
use crate::process::Process;
use crate::ref_counted::SharedRef;
use crate::scheduler::GLOBAL_SCHEDULER;
use crate::spinlock::SpinLock;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Runnable,
    Blocked,
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
        pc: fn(usize),
        arg: usize,
    ) -> SharedRef<Thread> {
        let thread = SharedRef::new(Thread {
            id: ThreadId::alloc(),
            mutable: SpinLock::new(Mutable {
                state: State::Runnable,
            }),
            arch: arch::Thread::new_kernel(pc as usize, arg),
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

    pub fn resume(&self) -> ! {
        self.arch.resume();
    }

    pub fn set_blocked(self: &SharedRef<Thread>) {
        let mut mutable = self.mutable.lock();
        debug_assert!(matches!(mutable.state, State::Runnable));

        mutable.state = State::Blocked;
    }

    pub fn set_runnable(self: &SharedRef<Thread>) {
        let mut mutable = self.mutable.lock();
        debug_assert!(matches!(mutable.state, State::Blocked));

        mutable.state = State::Runnable;
        GLOBAL_SCHEDULER.push(self.clone());
    }
}
