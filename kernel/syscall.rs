use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::handle::HandleRights;
use starina::message::MessageInfo;
use starina::poll::Readiness;
use starina::syscall::InKernelSyscallTable;

use crate::arch::enter_kernelland;
use crate::channel::Channel;
use crate::cpuvar::current_thread;
use crate::handle::Handle;
use crate::poll::Poll;
use crate::refcount::SharedRef;
use crate::thread::Thread;
use crate::thread::ThreadState;
use crate::thread::switch_thread;

#[unsafe(no_mangle)]
static INKERNEL_SYSCALL_TABLE: InKernelSyscallTable = InKernelSyscallTable {
    console_write: crate::arch::console_write,
    thread_yield: thread_yield_trampoline,
    poll_create: poll_create_trampoline,
    poll_add: poll_add_trampoline,
    poll_wait: poll_wait_trampoline,
    channel_send: channel_send_trampoline,
    channel_recv: channel_recv_trampoline,
};

type SyscallResult = Result<ThreadState, ErrorCode>;

fn thread_yield_trampoline() {
    enter_kernelland(123, 0, 0, 0, 0, 0);
}

fn kernel_scope<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    unsafe {
        use core::arch::asm;
        #[cfg(target_arch = "riscv64")]
        asm!("csrrw tp, sscratch, tp");
        let ret = f();
        #[cfg(target_arch = "riscv64")]
        asm!("csrrw tp, sscratch, tp");
        ret
    }
}

fn poll_create_trampoline() -> Result<HandleId, ErrorCode> {
    kernel_scope(|| {
        let mut poll = Poll::new()?;
        let handle = Handle::new(poll, HandleRights::POLL | HandleRights::WRITE);
        let current_thread = current_thread();
        let poll_id = current_thread.process().handles().lock().insert(handle)?;
        Ok(poll_id)
    })
}

fn poll_add_trampoline(
    poll: HandleId,
    object: HandleId,
    interests: Readiness,
) -> Result<(), ErrorCode> {
    kernel_scope(|| {
        let current_thread = current_thread();
        poll_add(&current_thread, poll, object, interests)
    })
}

fn poll_add(
    current: &SharedRef<Thread>,
    poll: HandleId,
    object: HandleId,
    interests: Readiness,
) -> Result<(), ErrorCode> {
    let handles = current.process().handles().lock();
    println!(
        "poll_add: poll={:?}, object={:?}, interests={:?}",
        poll, object, interests
    );
    let poll = handles.get::<Poll>(poll)?;
    let object_handle = handles.get_any(object)?;

    if !poll.is_capable(HandleRights::WRITE) {
        return Err(ErrorCode::NotAllowed);
    }

    poll.add(object_handle, object, interests)?;
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

fn channel_send_trampoline(
    handle: HandleId,
    data: *const u8,
    handles: *const HandleId,
) -> Result<(), ErrorCode> {
    kernel_scope(|| {
        let current_thread = current_thread();
        channel_send(&current_thread, handle, data, handles)
    })
}

fn channel_send(
    current: &SharedRef<Thread>,
    handle: HandleId,
    data: *const u8,
    handles: *const HandleId,
) -> Result<(), ErrorCode> {
    kernel_scope(|| {
        let handles = current.process().handles().lock();
        let ch = handles.get::<Channel>(handle)?;

        if !ch.is_capable(HandleRights::WRITE) {
            return Err(ErrorCode::NotAllowed);
        }

        ch.send(todo!(), todo!(), todo!())
    })
}

fn channel_recv_trampoline(
    handle: HandleId,
    data: *mut u8,
    handles: *mut HandleId,
) -> Result<MessageInfo, ErrorCode> {
    kernel_scope(|| {
        let current_thread = current_thread();
        channel_recv(&current_thread, handle, data, handles)
    })
}

fn channel_recv(
    current: &SharedRef<Thread>,
    handle: HandleId,
    data: *mut u8,
    handles: *mut HandleId,
) -> Result<MessageInfo, ErrorCode> {
    let handles = current.process().handles().lock();
    let ch = handles.get::<Channel>(handle)?;

    if !ch.is_capable(HandleRights::READ) {
        return Err(ErrorCode::NotAllowed);
    }

    let msginfo = ch.recv(todo!(), todo!())?;
    Ok(msginfo)
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
