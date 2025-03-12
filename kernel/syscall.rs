use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::handle::HandleRights;
use starina::message::MESSAGE_DATA_LEN_MAX;
use starina::message::MESSAGE_NUM_HANDLES_MAX;
use starina::message::MessageInfo;
use starina::poll::Readiness;
use starina::syscall::InKernelSyscallTable;
use starina::syscall::RetVal;
use starina::syscall::SyscallNumber;

use crate::arch::enter_kernelland;
use crate::channel::Channel;
use crate::cpuvar::current_thread;
use crate::handle::Handle;
use crate::isolation::IsolationHeap;
use crate::isolation::IsolationHeapMut;
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
    handle_close: handle_close_trampoline,
};

type SyscallResult = Result<ThreadState, ErrorCode>;

pub enum BlockableSyscallResult<T: Into<RetVal>> {
    Blocked(ThreadState),
    Done(Result<T, ErrorCode>),
}

fn handle_close(handle: HandleId) -> Result<(), ErrorCode> {
    let current_thread = current_thread();
    let mut handle_table = current_thread.process().handles().lock();
    handle_table.close(handle)?;
    Ok(())
}

fn handle_close_trampoline(handle: HandleId) -> Result<(), ErrorCode> {
    kernel_scope(|| handle_close(handle))
}

fn thread_yield_trampoline() {
    enter_kernelland(0, 0, 0, 0, 0, SyscallNumber::ThreadYield as isize);
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
        let poll = Poll::new()?;
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
    enter_kernelland(
        poll.as_raw() as isize,
        0,
        0,
        0,
        0,
        SyscallNumber::PollWait as isize,
    )
    .into()
}

fn poll_wait(current: &SharedRef<Thread>, poll: HandleId) -> SyscallResult {
    let handles = current.process().handles().lock();
    let poll = handles.get::<Poll>(poll)?;

    if !poll.is_capable(HandleRights::POLL) {
        return Err(ErrorCode::NotAllowed);
    }

    match poll.try_wait(current) {
        BlockableSyscallResult::Done(result) => Ok(ThreadState::Runnable(Some(result.into()))),
        BlockableSyscallResult::Blocked(state) => Ok(state),
    }
}

fn channel_send_trampoline(
    ch: HandleId,
    msginfo: MessageInfo,
    data: *const u8,
    handles: *const HandleId,
) -> Result<(), ErrorCode> {
    kernel_scope(|| {
        let current_thread = current_thread();
        channel_send(&current_thread, ch, msginfo, data, handles)
    })
}

fn channel_send(
    current: &SharedRef<Thread>,
    ch: HandleId,
    msginfo: MessageInfo,
    data: *const u8,
    handles: *const HandleId,
) -> Result<(), ErrorCode> {
    let mut handle_table = current.process().handles().lock();
    let ch = handle_table.get::<Channel>(ch)?;

    if !ch.is_capable(HandleRights::WRITE) {
        return Err(ErrorCode::NotAllowed);
    }

    let data = IsolationHeap::InKernel {
        ptr: data,
        len: msginfo.data_len(),
    };
    let handles = IsolationHeap::InKernel {
        ptr: handles as *const u8,
        len: msginfo.num_handles(),
    };
    ch.send(&mut handle_table, msginfo, &data, &handles)
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
    let mut handle_table = current.process().handles().lock();
    let ch = handle_table.get::<Channel>(handle)?;

    if !ch.is_capable(HandleRights::READ) {
        return Err(ErrorCode::NotAllowed);
    }

    let mut data = IsolationHeapMut::InKernel {
        ptr: data,
        len: MESSAGE_DATA_LEN_MAX,
    };
    let mut handles = IsolationHeapMut::InKernel {
        ptr: handles as *mut u8,
        len: MESSAGE_NUM_HANDLES_MAX,
    };
    let msginfo = ch.recv(&mut handle_table, &mut data, &mut handles)?;
    Ok(msginfo)
}

pub fn do_syscall(
    a0: isize,
    _a1: isize,
    _a2: isize,
    _a3: isize,
    _a4: isize,
    a5: isize,
    current: &SharedRef<Thread>,
) -> SyscallResult {
    if a5 == SyscallNumber::PollWait as isize {
        let poll = HandleId::from_raw(a0.try_into().unwrap()); // FIXME:
        poll_wait(current, poll)
    } else {
        warn!("unknown syscall: {}", a5);
        Err(ErrorCode::InvalidSyscall)
    }
}

pub extern "C" fn syscall_handler(
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    a5: isize,
) -> ! {
    trace!(
        "syscall_handler: a0={:x}, a1={:x}, a2={:x}, a3={:x}, a4={:x}, a5={:x}",
        a0, a1, a2, a3, a4, a5
    );

    {
        let current = current_thread();
        let result = do_syscall(a0, a1, a2, a3, a4, a5, &current);
        let state = match result {
            Ok(state) => state,
            Err(err) => ThreadState::Runnable(Some(err.into())),
        };
        current.set_state(state);
    }

    switch_thread();
}
