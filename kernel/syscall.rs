use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
use starina_types::handle::HandleRights;
use starina_types::message::MESSAGE_DATA_LEN_MAX;
use starina_types::message::MESSAGE_NUM_HANDLES_MAX;
use starina_types::message::MessageInfo;
use starina_types::poll::Readiness;
use starina_types::syscall::*;

use crate::arch;
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

pub enum SyscallResult {
    Done(RetVal),
    Err(ErrorCode),
    Block(ThreadState),
}

fn handle_close(current: &SharedRef<Thread>, handle: HandleId) -> Result<(), ErrorCode> {
    let mut handle_table = current.process().handles().lock();
    handle_table.close(handle)?;
    Ok(())
}

fn poll_create(current: &SharedRef<Thread>) -> Result<HandleId, ErrorCode> {
    let poll = Poll::new()?;
    let handle = Handle::new(poll, HandleRights::POLL | HandleRights::WRITE);
    let poll_id = current.process().handles().lock().insert(handle)?;
    Ok(poll_id)
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

fn poll_wait(current: &SharedRef<Thread>, poll: HandleId) -> SyscallResult {
    let handles = current.process().handles().lock();
    let poll = match handles.get::<Poll>(poll) {
        Ok(poll) => poll,
        Err(e) => {
            return SyscallResult::Err(e);
        }
    };

    if !poll.is_capable(HandleRights::POLL) {
        return SyscallResult::Err(ErrorCode::NotAllowed);
    }

    poll.try_wait(current)
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

fn do_syscall(
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    n: isize,
    current: &SharedRef<Thread>,
) -> Result<SyscallResult, ErrorCode> {
    match n as u8 {
        SYS_HANDLE_CLOSE => {
            let handle = HandleId::from_raw_isize(a0)?;
            let ret = handle_close(&current, handle)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_CONSOLE_WRITE => {
            // FIXME: use isolation heap
            let buf = unsafe { core::slice::from_raw_parts(a0 as *const u8, a1 as usize) };
            arch::console_write(buf);
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_POLL_CREATE => {
            let poll = poll_create(&current)?;
            Ok(SyscallResult::Done(poll.into()))
        }
        SYS_POLL_ADD => {
            let poll = HandleId::from_raw_isize(a0)?;
            let object = HandleId::from_raw_isize(a1)?;
            let interests = Readiness::from_raw_isize(a2)?;
            let ret = poll_add(&current, poll, object, interests)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_POLL_WAIT => {
            let poll = HandleId::from_raw_isize(a0)?;
            let ret = poll_wait(&current, poll);
            Ok(ret)
        }
        SYS_CHANNEL_SEND => {
            let ch = HandleId::from_raw_isize(a0)?;
            let msginfo = MessageInfo::from_raw_isize(a1)?;
            let data = a2 as *const u8;
            let handles = a3 as *const HandleId;
            channel_send(&current, ch, msginfo, data, handles)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_CHANNEL_RECV => {
            let handle = HandleId::from_raw_isize(a0)?;
            let data = a1 as *mut u8;
            let handles = a2 as *mut HandleId;
            let msginfo = channel_recv(&current, handle, data, handles)?;
            Ok(SyscallResult::Done(msginfo.into()))
        }
        _ => {
            debug_warn!("unknown syscall: {}", n);
            Err(ErrorCode::InvalidSyscall)
        }
    }
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

pub extern "C" fn syscall_inkernel_handler(
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    n: isize,
) -> ! {
    kernel_scope(|| {
        let current = current_thread();
        let new_state = match do_syscall(a0, a1, a2, a3, a4, n, &current) {
            Ok(SyscallResult::Done(value)) => ThreadState::Runnable(Some(value.into())),
            Ok(SyscallResult::Err(err)) => ThreadState::Runnable(Some(err.into())),
            Ok(SyscallResult::Block(state)) => state,
            Err(err) => ThreadState::Runnable(None),
        };
        current.set_state(new_state);
    });

    switch_thread();
}
