use core::ops::Deref;

use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::handle::HandleRights;
use starina::poll::Readiness;
use starina::syscall::InKernelSyscallTable;

use crate::arch::enter_kernelland;
use crate::cpuvar::current_thread;
use crate::poll::Poll;
use crate::refcount::SharedRef;
use crate::thread::Thread;
use crate::thread::ThreadState;
use crate::thread::switch_thread;

#[unsafe(no_mangle)]
static INKERNEL_SYSCALL_TABLE: InKernelSyscallTable = InKernelSyscallTable {
    console_write: crate::arch::console_write,
    thread_yield: thread_yield_trampoline,
    poll_add: poll_add_trampoline,
    poll_wait: poll_wait_trampoline,
};

type SyscallResult = Result<ThreadState, ErrorCode>;

fn thread_yield_trampoline() {
    enter_kernelland(123, 0, 0, 0, 0, 0);
}

fn poll_add_trampoline(
    poll: HandleId,
    object: HandleId,
    interests: Readiness,
) -> Result<(), ErrorCode> {
    let current_thread = current_thread();
    poll_add(&current_thread, poll, object, interests)
}

fn poll_add(
    current: &SharedRef<Thread>,
    poll: HandleId,
    object: HandleId,
    interests: Readiness,
) -> Result<(), ErrorCode> {
    let handles = current.process().handles().lock();
    let poll = handles.get::<Poll>(poll)?;

    if !poll.is_capable(HandleRights::WRITE) {
        return Err(ErrorCode::NotAllowed);
    }

    Ok(())
}

fn poll_wait_trampoline(poll: HandleId) -> Result<(HandleId, Readiness), ErrorCode> {
    let result = enter_kernelland(123, 0, 0, 0, 0, 0);
    todo!()
}

fn poll_wait(current: &SharedRef<Thread>, poll: HandleId) -> SyscallResult {
    let handles = current.process().handles().lock();
    let poll = handles.get::<Poll>(poll)?;

    if !poll.is_capable(HandleRights::POLL) {
        return Err(ErrorCode::NotAllowed);
    }

    Ok(ThreadState::BlockedByPoll(poll.into_object()))
}

#[derive(Debug, Clone, Copy)]
pub struct RetVal(isize);

impl RetVal {
    pub const fn new(value: isize) -> RetVal {
        RetVal(value)
    }

    pub fn as_isize(&self) -> isize {
        self.0
    }
}

impl<T> From<Result<T, ErrorCode>> for RetVal
where
    T: Into<RetVal>,
{
    fn from(value: Result<T, ErrorCode>) -> Self {
        match value {
            Ok(value) => value.into(),
            Err(err) => RetVal(err as isize),
        }
    }
}

impl From<(HandleId, Readiness)> for RetVal {
    fn from(value: (HandleId, Readiness)) -> Self {
        todo!()
    }
}

pub extern "C" fn syscall_handler(
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
) -> ! {
    trace!(
        "syscall_handler: a0={:x}, a1={:x}, a2={:x}, a3={:x}, a4={:x}, a5={:x}",
        a0, a1, a2, a3, a4, a5
    );

    // let current_thread = current_thread();
    // syscall
    // current_thread.set_state(new_state);

    switch_thread();
}
