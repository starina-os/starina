use starina::address::DAddr;
use starina::address::GPAddr;
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
use starina_types::vcpu::VCpuExit;
use starina_types::vmspace::PageProtect;

use crate::arch;
use crate::arch::inkernel_syscall_entry;
use crate::channel::Channel;
use crate::cpuvar::current_thread;
use crate::folio::Folio;
use crate::handle::Handle;
use crate::hvspace::HvSpace;
use crate::interrupt::Interrupt;
use crate::iobus::IoBus;
use crate::isolation::IsolationHeap;
use crate::isolation::IsolationHeapMut;
use crate::poll::Poll;
use crate::refcount::SharedRef;
use crate::thread::Thread;
use crate::thread::ThreadState;
use crate::thread::switch_thread;
use crate::vcpu::VCpu;
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

fn poll_remove(
    current: &SharedRef<Thread>,
    poll: HandleId,
    object: HandleId,
) -> Result<(), ErrorCode> {
    let handles = current.process().handles().lock();
    let poll = handles.get::<Poll>(poll)?;

    if !poll.is_capable(HandleRights::WRITE) {
        return Err(ErrorCode::NotAllowed);
    }

    poll.remove(object)?;
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

pub fn folio_alloc(current: &SharedRef<Thread>, len: usize) -> Result<HandleId, ErrorCode> {
    let folio = Folio::alloc(len)?;
    let handle: Handle<Folio> = Handle::new(
        SharedRef::new(folio)?,
        HandleRights::READ | HandleRights::WRITE | HandleRights::MAP,
    );
    let handle_id = current.process().handles().lock().insert(handle)?;
    Ok(handle_id)
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
    let handle: Handle<Interrupt> =
        Handle::new(interrupt, HandleRights::READ | HandleRights::WRITE);
    let handle_id = current.process().handles().lock().insert(handle)?;
    Ok(handle_id)
}

fn interrupt_ack(current: &SharedRef<Thread>, handle: HandleId) -> Result<(), ErrorCode> {
    let mut handle_table = current.process().handles().lock();
    let interrupt = handle_table.get::<Interrupt>(handle)?;
    interrupt.acknowledge()?;
    Ok(())
}

fn hvspace_create(current: &SharedRef<Thread>) -> Result<HandleId, ErrorCode> {
    let hvspace = HvSpace::new()?;
    let handle = Handle::new(
        SharedRef::new(hvspace)?,
        HandleRights::READ | HandleRights::WRITE,
    );
    let mut handle_table = current.process().handles().lock();
    let handle_id = handle_table.insert(handle)?;
    Ok(handle_id)
}

fn hvspace_map(
    current: &SharedRef<Thread>,
    hvspace_handle: HandleId,
    gpaddr: GPAddr,
    folio_handle: HandleId,
    len: usize,
    prot: PageProtect,
) -> Result<(), ErrorCode> {
    let mut handle_table = current.process().handles().lock();
    let mut hvspace = handle_table.get::<HvSpace>(hvspace_handle)?;
    let folio = handle_table.get::<Folio>(folio_handle)?;
    hvspace.map(gpaddr, folio.into_object(), len, prot)?;
    Ok(())
}

fn vcpu_create(
    current: &SharedRef<Thread>,
    hvspace_handle: HandleId,
    entry: usize,
    arg0: usize,
    arg1: usize,
) -> Result<HandleId, ErrorCode> {
    let mut handle_table = current.process().handles().lock();
    let hvspace = handle_table.get::<HvSpace>(hvspace_handle)?;
    let vcpu = VCpu::new(hvspace.into_object(), entry, arg0, arg1)?;
    let handle = Handle::new(SharedRef::new(vcpu)?, HandleRights::EXEC);
    let handle_id = handle_table.insert(handle)?;
    Ok(handle_id)
}

fn vcpu_run(
    current: &SharedRef<Thread>,
    vcpu_handle: HandleId,
    exit: IsolationHeapMut,
) -> Result<ThreadState, ErrorCode> {
    let mut handle_table = current.process().handles().lock();
    let vcpu = handle_table.get::<VCpu>(vcpu_handle)?;
    if !vcpu.is_capable(HandleRights::EXEC) {
        return Err(ErrorCode::NotAllowed);
    }

    // FIXME: Better isolation heap API
    vcpu.update(exit)?;

    let new_state = ThreadState::RunVCpu(vcpu.into_object());
    Ok(new_state)
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
        SYS_THREAD_EXIT => {
            debug_warn!("thread exit");
            Ok(SyscallResult::Block(ThreadState::Exited))
        }
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
        SYS_POLL_REMOVE => {
            let poll = HandleId::from_raw_isize(a0)?;
            let object = HandleId::from_raw_isize(a1)?;
            poll_remove(&current, poll, object)?;
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
        SYS_FOLIO_ALLOC => {
            let len = a0 as usize;
            let ret = folio_alloc(&current, len)?;
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
        SYS_HVSPACE_CREATE => {
            let ret = hvspace_create(&current)?;
            Ok(SyscallResult::Done(ret.into()))
        }
        SYS_HVSPACE_MAP => {
            let hvspace_handle = HandleId::from_raw_isize(a0)?;
            let gpaddr = GPAddr::new(a1 as usize);
            let folio_handle = HandleId::from_raw_isize(a2)?;
            let len = a3 as usize;
            let prot = PageProtect::from_raw_isize(a4)?;
            hvspace_map(&current, hvspace_handle, gpaddr, folio_handle, len, prot)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_VCPU_CREATE => {
            let hvspace_handle = HandleId::from_raw_isize(a0)?;
            let entry = a1 as usize;
            let arg0 = a2 as usize;
            let arg1 = a3 as usize;
            let ret = vcpu_create(&current, hvspace_handle, entry, arg0, arg1)?;
            Ok(SyscallResult::Done(ret.into()))
        }
        SYS_VCPU_RUN => {
            let vcpu_handle = HandleId::from_raw_isize(a0)?;
            let exit = IsolationHeapMut::InKernel {
                ptr: a1 as *mut u8,
                len: size_of::<VCpuExit>(),
            };
            let new_state = vcpu_run(&current, vcpu_handle, exit)?;
            Ok(SyscallResult::Block(new_state))
        }
        _ => {
            debug_warn!("unknown syscall: {}", n);
            Err(ErrorCode::InvalidSyscall)
        }
    }
}

pub fn syscall_handler(a0: isize, a1: isize, a2: isize, a3: isize, a4: isize, n: isize) -> ! {
    let current = current_thread();
    let new_state = match do_syscall(a0, a1, a2, a3, a4, n, &current) {
        Ok(SyscallResult::Done(value)) => ThreadState::Runnable(Some(value.into())),
        Ok(SyscallResult::Err(err)) => ThreadState::Runnable(Some(err.into())),
        Ok(SyscallResult::Block(state)) => state,
        Err(err) => ThreadState::Runnable(Some(err.into())),
    };
    current.set_state(new_state);
    drop(current);

    switch_thread();
}
