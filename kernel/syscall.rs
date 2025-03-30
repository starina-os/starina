use starina::address::DAddr;
use starina::address::VAddr;
use starina::interrupt::Irq;
use starina::interrupt::IrqMatcher;
use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
use starina_types::handle::HandleRights;
use starina_types::message::MESSAGE_DATA_LEN_MAX;
use starina_types::message::MESSAGE_NUM_HANDLES_MAX;
use starina_types::message::MessageInfo;
use starina_types::poll::Readiness;
use starina_types::syscall::*;
use starina_types::vmspace::PageProtect;

use crate::arch;
use crate::arch::enter_kernelland;
use crate::channel::Channel;
use crate::cpuvar::current_thread;
use crate::folio::Folio;
use crate::handle::Handle;
use crate::interrupt::Interrupt;
use crate::iobus::IoBus;
use crate::isolation::IsolationHeap;
use crate::isolation::IsolationHeapMut;
use crate::poll::Poll;
use crate::refcount::SharedRef;
use crate::thread::Thread;
use crate::thread::ThreadState;
use crate::thread::switch_thread;
use crate::vmspace::VmSpace;

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

fn channel_create(current: &SharedRef<Thread>) -> Result<HandleId, ErrorCode> {
    let (ch1, ch2) = Channel::new()?;
    let handle_table = &mut current.process().handles().lock();
    let ch1_handle = Handle::new(ch1, HandleRights::READ | HandleRights::WRITE);
    let ch2_handle = Handle::new(ch2, HandleRights::READ | HandleRights::WRITE);
    let ch1_id = handle_table.insert(ch1_handle)?;
    let ch2_id = handle_table.insert(ch2_handle)?;
    assert!((ch1_id.as_raw() + 1) == ch2_id.as_raw()); // FIXME: guarantee this in HandleTable
    Ok((ch1_id))
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

pub fn vmspace_map(
    current: &SharedRef<Thread>,
    handle: HandleId,
    folio: HandleId,
    prot: PageProtect,
) -> Result<VAddr, ErrorCode> {
    let mut handle_table = current.process().handles().lock();
    let vmspace = if handle.as_raw() == 0 {
        current.process().vmspace().clone()
    } else {
        let handle = handle_table.get::<VmSpace>(handle)?;
        if !handle.is_capable(HandleRights::WRITE) {
            return Err(ErrorCode::NotAllowed);
        }

        handle.into_object()
    };

    let folio = handle_table.get::<Folio>(folio)?;
    if !folio.is_capable(HandleRights::MAP) {
        return Err(ErrorCode::NotAllowed);
    }

    let vaddr = vmspace.map_anywhere(folio.into_object(), prot)?;
    Ok(vaddr)
}

pub fn folio_daddr(current: &SharedRef<Thread>, handle: HandleId) -> Result<DAddr, ErrorCode> {
    let mut handle_table = current.process().handles().lock();
    let folio = handle_table.get::<Folio>(handle)?;
    let daddr = folio.daddr().ok_or(ErrorCode::NotADevice)?;
    Ok(daddr)
}

fn busio_map(
    current: &SharedRef<Thread>,
    handle: HandleId,
    daddr: Option<DAddr>,
    len: usize,
) -> Result<HandleId, ErrorCode> {
    let mut handle_table = current.process().handles().lock();
    let busio = handle_table.get::<IoBus>(handle)?;
    if !busio.is_capable(HandleRights::WRITE) {
        return Err(ErrorCode::NotAllowed);
    }

    let folio = busio.map(daddr, len)?;
    let handle = Handle::new(SharedRef::new(folio)?, HandleRights::MAP);
    let folio_id = handle_table.insert(handle)?;
    Ok(folio_id)
}

fn interrupt_create(
    current: &SharedRef<Thread>,
    irq_matcher: IrqMatcher,
) -> Result<HandleId, ErrorCode> {
    let interrupt = Interrupt::attach(irq_matcher)?;
    let handle = Handle::new(interrupt, HandleRights::READ | HandleRights::WRITE);
    let handle_id = current.process().handles().lock().insert(handle)?;
    Ok(handle_id)
}

fn interrupt_ack(current: &SharedRef<Thread>, handle: HandleId) -> Result<(), ErrorCode> {
    let mut handle_table = current.process().handles().lock();
    let interrupt = handle_table.get::<Interrupt>(handle)?;
    interrupt.acknowledge()?;
    Ok(())
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
            poll_add(&current, poll, object, interests)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_POLL_WAIT => {
            let poll = HandleId::from_raw_isize(a0)?;
            let ret = poll_wait(&current, poll);
            Ok(ret)
        }
        SYS_CHANNEL_CREATE => {
            let ch1 = channel_create(&current)?;
            Ok(SyscallResult::Done((ch1.into())))
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
        SYS_VMSPACE_MAP => {
            let handle = HandleId::from_raw_isize(a0)?;
            let folio = HandleId::from_raw_isize(a1)?;
            let prot = PageProtect::from_raw_isize(a2)?;
            let ret = vmspace_map(&current, handle, folio, prot)?;
            Ok(SyscallResult::Done(ret.into()))
        }
        SYS_FOLIO_DADDR => {
            let handle = HandleId::from_raw_isize(a0)?;
            let daddr = folio_daddr(&current, handle)?;
            Ok(SyscallResult::Done(daddr.into()))
        }
        SYS_BUSIO_MAP => {
            let handle = HandleId::from_raw_isize(a0)?;
            let daddr = if a1 == 0 {
                None
            } else {
                Some(DAddr::new(a1 as usize))
            };
            let len = a2 as usize;
            let ret = busio_map(&current, handle, daddr, len)?;
            Ok(SyscallResult::Done(ret.into()))
        }
        SYS_INTERRUPT_CREATE => {
            let irq_matcher = IrqMatcher::from_raw_isize(a0)?;
            let ret = interrupt_create(&current, irq_matcher)?;
            Ok(SyscallResult::Done(ret.into()))
        }
        SYS_INTERRUPT_ACK => {
            let handle = HandleId::from_raw_isize(a0)?;
            interrupt_ack(&current, handle)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        _ => {
            debug_warn!("unknown syscall: {}", n);
            Err(ErrorCode::InvalidSyscall)
        }
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
    arch::kernel_scope(|| {
        let current = current_thread();
        let new_state = match do_syscall(a0, a1, a2, a3, a4, n, &current) {
            Ok(SyscallResult::Done(value)) => ThreadState::Runnable(Some(value.into())),
            Ok(SyscallResult::Err(err)) => ThreadState::Runnable(Some(err.into())),
            Ok(SyscallResult::Block(state)) => state,
            Err(err) => ThreadState::Runnable(Some(err.into())),
        };
        current.set_state(new_state);
    });

    switch_thread();
}
