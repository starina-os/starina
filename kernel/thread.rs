use core::num::NonZeroIsize;
use core::sync::atomic::AtomicIsize;
use core::sync::atomic::Ordering;

use crate::arch::{self};
use crate::handle::Handleable;
use crate::ref_counted::SharedRef;
use crate::scheduler::GLOBAL_SCHEDULER;

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
}

pub struct Thread {
    id: ThreadId,
    state: State,
    arch: arch::Thread,
}

impl Thread {
    pub fn test() -> Thread {
        Thread {
            id: ThreadId::new_idle(),
            state: State::Runnable,
            arch: arch::Thread::new_idle(),
        }
    }

    pub fn new_idle() -> SharedRef<Thread> {
        SharedRef::new(Thread {
            id: ThreadId::new_idle(),
            state: State::Runnable,
            arch: arch::Thread::new_idle(),
        })
    }

    pub fn spawn_kernel(pc: fn(usize), arg: usize) -> SharedRef<Thread> {
        let thread = SharedRef::new(Thread {
            id: ThreadId::alloc(),
            state: State::Runnable,
            arch: arch::Thread::new_kernel(pc as usize, arg),
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
        matches!(self.state, State::Runnable)
    }

    pub const fn arch(&self) -> &arch::Thread {
        &self.arch
    }

    pub fn resume(&self) -> ! {
        self.arch.resume();
    }
}

impl Handleable for Thread {}
