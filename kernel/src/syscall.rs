use core::cmp::min;

use starina::address::GPAddr;
use starina::address::PAddr;
use starina::address::VAddr;
use starina::interrupt::IrqMatcher;
use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
use starina_types::handle::HandleRights;
use starina_types::message::MESSAGE_DATA_LEN_MAX;
use starina_types::message::MESSAGE_NUM_HANDLES_MAX;
use starina_types::message::MessageInfo;
use starina_types::poll::Readiness;
use starina_types::syscall::*;
use starina_types::vcpu::VCpuRunState;
use starina_types::vmspace::PageProtect;

use crate::arch;
use crate::channel::Channel;
use crate::cpuvar::current_thread;
use crate::folio::Folio;
use crate::handle::Handle;
use crate::hvspace::HvSpace;
use crate::interrupt::Interrupt;
use crate::isolation::IsolationPtr;
use crate::isolation::IsolationSlice;
use crate::isolation::IsolationSliceMut;
use crate::poll::Poll;
use crate::process::KERNEL_PROCESS;
use crate::refcount::SharedRef;
use crate::thread::Thread;
use crate::thread::ThreadState;
use crate::thread::switch_thread;
use crate::timer::Timer;
use crate::vcpu::VCpu;
use crate::vmspace::VmSpace;

pub enum SyscallResult {
    Done(RetVal),
    Err(ErrorCode),
    Block(ThreadState),
}

fn console_write(
    current: &SharedRef<Thread>,
    str_ptr: IsolationPtr,
    len: usize,
) -> Result<(), ErrorCode> {
    let slice = IsolationSlice::new(str_ptr, len);
    let isolation = current.process().isolation();

    let mut tmp = [0u8; 512];
    let mut remaining = len;
    let mut offset = 0;

    while remaining > 0 {
        let chunk_len = min(remaining, tmp.len());
        let buf = &mut tmp[..chunk_len];

        slice.read_to_slice(isolation, offset, buf)?;
        arch::console_write(buf);

        offset += chunk_len;
        remaining -= chunk_len;
    }

    Ok(())
}

fn thread_spawn(
    current: &SharedRef<Thread>,
    process_handle: HandleId,
    pc: usize,
    arg: usize,
) -> Result<HandleId, ErrorCode> {
    if process_handle.as_raw() != 0 {
        debug_warn!("thread_spawn syscall does not support process != 0");
        return Err(ErrorCode::NotSupported);
    }

    let process = current.process();
    if !SharedRef::ptr_eq(process, &KERNEL_PROCESS) {
        debug_warn!("thread_spawn syscall supports only in-kernel process (for now)");
        return Err(ErrorCode::NotSupported);
    }

    let thread = Thread::new_inkernel(pc, arg)?;
    let handle = Handle::new(thread, HandleRights::READ | HandleRights::WRITE);

    let handle_id = process.handles().lock().insert(handle)?;
    Ok(handle_id)
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

fn poll_update(
    current: &SharedRef<Thread>,
    poll: HandleId,
    object: HandleId,
    or_mask: Readiness,
    and_mask: Readiness,
) -> Result<(), ErrorCode> {
    let handles = current.process().handles().lock();
    let poll = handles.get::<Poll>(poll)?;

    if !poll.is_capable(HandleRights::WRITE) {
        return Err(ErrorCode::NotAllowed);
    }

    poll.update(object, or_mask, and_mask)?;
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

    poll.try_wait(current, false)
}

fn poll_try_wait(current: &SharedRef<Thread>, poll: HandleId) -> SyscallResult {
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

    poll.try_wait(current, true)
}

fn channel_create(current: &SharedRef<Thread>) -> Result<HandleId, ErrorCode> {
    let (ch1, ch2) = Channel::new()?;
    let handle_table = &mut current.process().handles().lock();
    let ch1_handle = Handle::new(ch1, HandleRights::READ | HandleRights::WRITE);
    let ch2_handle = Handle::new(ch2, HandleRights::READ | HandleRights::WRITE);
    let ch1_id = handle_table.insert_consecutive(ch1_handle, ch2_handle)?;
    Ok(ch1_id)
}

fn channel_send(
    current: &SharedRef<Thread>,
    ch: HandleId,
    msginfo: MessageInfo,
    data_ptr: IsolationPtr,
    handles_ptr: IsolationPtr,
) -> Result<(), ErrorCode> {
    let process = current.process();
    let isolation = process.isolation();
    let mut handle_table = process.handles().lock();
    let ch = handle_table.get::<Channel>(ch)?;

    if !ch.is_capable(HandleRights::WRITE) {
        return Err(ErrorCode::NotAllowed);
    }

    let data = IsolationSlice::new(data_ptr, msginfo.data_len());
    let handles = IsolationSlice::new(handles_ptr, size_of::<HandleId>() * msginfo.num_handles());
    ch.send(isolation, &mut handle_table, msginfo, data, handles)
}

fn channel_recv(
    current: &SharedRef<Thread>,
    handle: HandleId,
    data_ptr: IsolationPtr,
    handles_ptr: IsolationPtr,
) -> Result<MessageInfo, ErrorCode> {
    let process = current.process();
    let isolation = process.isolation();
    let mut handle_table = process.handles().lock();
    let ch = handle_table.get::<Channel>(handle)?;

    if !ch.is_capable(HandleRights::READ) {
        return Err(ErrorCode::NotAllowed);
    }

    let data = IsolationSliceMut::new(data_ptr, MESSAGE_DATA_LEN_MAX);
    let handles =
        IsolationSliceMut::new(handles_ptr, size_of::<HandleId>() * MESSAGE_NUM_HANDLES_MAX);
    let msginfo = ch.recv(isolation, &mut handle_table, data, handles)?;
    Ok(msginfo)
}

pub fn vmspace_map(
    current: &SharedRef<Thread>,
    handle: HandleId,
    vaddr: VAddr,
    len: usize,
    folio: HandleId,
    offset: usize,
    prot: PageProtect,
) -> Result<VAddr, ErrorCode> {
    let process = current.process();
    let handle_table = current.process().handles().lock();
    let vmspace = if handle.as_raw() == 0 {
        process.isolation().vmspace().clone()
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

    if vaddr != VAddr::new(0) {
        debug_warn!("vmspace_map syscall does not support vaddr != 0");
        return Err(ErrorCode::NotSupported);
    }

    if folio.len() != len {
        debug_warn!("vmspace_map syscall does not support folio.len != len");
        return Err(ErrorCode::NotSupported);
    }

    if offset != 0 {
        debug_warn!("vmspace_map syscall does not support offset != 0");
        return Err(ErrorCode::NotSupported);
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

pub fn folio_paddr(current: &SharedRef<Thread>, handle: HandleId) -> Result<PAddr, ErrorCode> {
    let handle_table = current.process().handles().lock();
    let folio = handle_table.get::<Folio>(handle)?;
    Ok(folio.paddr())
}

pub fn folio_pin(
    current: &SharedRef<Thread>,
    paddr: PAddr,
    len: usize,
) -> Result<HandleId, ErrorCode> {
    let folio = Folio::pin(paddr, len)?;
    let handle: Handle<Folio> = Handle::new(
        SharedRef::new(folio)?,
        HandleRights::READ | HandleRights::WRITE | HandleRights::MAP,
    );
    let handle_id = current.process().handles().lock().insert(handle)?;
    Ok(handle_id)
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
    let handle_table = current.process().handles().lock();
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
    let handle_table = current.process().handles().lock();
    let hvspace = handle_table.get::<HvSpace>(hvspace_handle)?;
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
    exit: IsolationSliceMut,
) -> Result<ThreadState, ErrorCode> {
    let handle_table = current.process().handles().lock();
    let vcpu = handle_table.get::<VCpu>(vcpu_handle)?;
    if !vcpu.is_capable(HandleRights::EXEC) {
        return Err(ErrorCode::NotAllowed);
    }

    let isolation = current.process().isolation();
    vcpu.apply_state(isolation, exit)?;

    let new_state = ThreadState::RunVCpu(vcpu.into_object());
    Ok(new_state)
}

fn timer_create(current: &SharedRef<Thread>) -> Result<HandleId, ErrorCode> {
    let timer = SharedRef::new(Timer::new())?;
    let handle = Handle::new(timer, HandleRights::READ | HandleRights::WRITE);
    let handle_id = current.process().handles().lock().insert(handle)?;
    Ok(handle_id)
}

fn timer_set(
    current: &SharedRef<Thread>,
    timer_handle: HandleId,
    duration_ns: u64,
) -> Result<(), ErrorCode> {
    let handle_table = current.process().handles().lock();
    let timer = handle_table.get::<Timer>(timer_handle)?;
    timer.set_timeout(duration_ns)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn do_syscall(
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    a5: isize,
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
            handle_close(current, handle)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_THREAD_SPAWN => {
            let process_handle = HandleId::from_raw_isize(a0)?;
            let pc = a1 as usize;
            let arg = a2 as usize;
            let thread = thread_spawn(current, process_handle, pc, arg)?;
            Ok(SyscallResult::Done(thread.into()))
        }
        SYS_CONSOLE_WRITE => {
            let str_ptr = IsolationPtr::new(a0 as usize);
            let len = a1 as usize;
            console_write(current, str_ptr, len)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_POLL_CREATE => {
            let poll = poll_create(current)?;
            Ok(SyscallResult::Done(poll.into()))
        }
        SYS_POLL_ADD => {
            let poll = HandleId::from_raw_isize(a0)?;
            let object = HandleId::from_raw_isize(a1)?;
            let interests = Readiness::from_raw_isize(a2)?;
            poll_add(current, poll, object, interests)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_POLL_UPDATE => {
            let poll = HandleId::from_raw_isize(a0)?;
            let object = HandleId::from_raw_isize(a1)?;
            let or_mask = Readiness::from_raw_isize(a2)?;
            let and_mask = Readiness::from_raw_isize(a3)?;
            poll_update(current, poll, object, or_mask, and_mask)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_POLL_REMOVE => {
            let poll = HandleId::from_raw_isize(a0)?;
            let object = HandleId::from_raw_isize(a1)?;
            poll_remove(current, poll, object)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_POLL_WAIT => {
            let poll = HandleId::from_raw_isize(a0)?;
            let ret = poll_wait(current, poll);
            Ok(ret)
        }
        SYS_POLL_TRY_WAIT => {
            let poll = HandleId::from_raw_isize(a0)?;
            let ret = poll_try_wait(current, poll);
            Ok(ret)
        }
        SYS_CHANNEL_CREATE => {
            let ch1 = channel_create(current)?;
            Ok(SyscallResult::Done(ch1.into()))
        }
        SYS_CHANNEL_SEND => {
            let ch = HandleId::from_raw_isize(a0)?;
            let msginfo = MessageInfo::from_raw_isize(a1)?;
            let data = IsolationPtr::new(a2 as usize);
            let handles = IsolationPtr::new(a3 as usize);
            channel_send(current, ch, msginfo, data, handles)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_CHANNEL_RECV => {
            let handle = HandleId::from_raw_isize(a0)?;
            let data_ptr = IsolationPtr::new(a1 as usize);
            let handles_ptr = IsolationPtr::new(a2 as usize);
            let msginfo = channel_recv(current, handle, data_ptr, handles_ptr)?;
            Ok(SyscallResult::Done(msginfo.into()))
        }
        SYS_VMSPACE_MAP => {
            let handle = HandleId::from_raw_isize(a0)?;
            let vaddr = VAddr::new(a1 as usize);
            let len = a2 as usize;
            let folio = HandleId::from_raw_isize(a3)?;
            let offset = a4 as usize;
            let prot = PageProtect::from_raw_isize(a5)?;
            let ret = vmspace_map(current, handle, vaddr, len, folio, offset, prot)?;
            Ok(SyscallResult::Done(ret.into()))
        }
        SYS_FOLIO_ALLOC => {
            let len = a0 as usize;
            let ret = folio_alloc(current, len)?;
            Ok(SyscallResult::Done(ret.into()))
        }
        SYS_FOLIO_PIN => {
            let paddr = PAddr::new(a0 as usize);
            let len = a1 as usize;
            let ret = folio_pin(current, paddr, len)?;
            Ok(SyscallResult::Done(ret.into()))
        }
        SYS_FOLIO_PADDR => {
            let handle = HandleId::from_raw_isize(a0)?;
            let ret = folio_paddr(current, handle)?;
            Ok(SyscallResult::Done(ret.into()))
        }
        SYS_INTERRUPT_CREATE => {
            let irq_matcher = IrqMatcher::from_raw_isize(a0)?;
            let ret = interrupt_create(current, irq_matcher)?;
            Ok(SyscallResult::Done(ret.into()))
        }
        SYS_INTERRUPT_ACK => {
            let handle = HandleId::from_raw_isize(a0)?;
            interrupt_ack(current, handle)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_HVSPACE_CREATE => {
            let ret = hvspace_create(current)?;
            Ok(SyscallResult::Done(ret.into()))
        }
        SYS_HVSPACE_MAP => {
            let hvspace_handle = HandleId::from_raw_isize(a0)?;
            let gpaddr = GPAddr::new(a1 as usize);
            let folio_handle = HandleId::from_raw_isize(a2)?;
            let len = a3 as usize;
            let prot = PageProtect::from_raw_isize(a4)?;
            hvspace_map(current, hvspace_handle, gpaddr, folio_handle, len, prot)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_VCPU_CREATE => {
            let hvspace_handle = HandleId::from_raw_isize(a0)?;
            let entry = a1 as usize;
            let arg0 = a2 as usize;
            let arg1 = a3 as usize;
            let ret = vcpu_create(current, hvspace_handle, entry, arg0, arg1)?;
            Ok(SyscallResult::Done(ret.into()))
        }
        SYS_VCPU_RUN => {
            let vcpu_handle = HandleId::from_raw_isize(a0)?;
            let exit_ptr = IsolationPtr::new(a1 as usize);
            let exit = IsolationSliceMut::new(exit_ptr, size_of::<VCpuRunState>());
            let new_state = vcpu_run(current, vcpu_handle, exit)?;
            Ok(SyscallResult::Block(new_state))
        }
        SYS_TIMER_CREATE => {
            let ret = timer_create(current)?;
            Ok(SyscallResult::Done(ret.into()))
        }
        SYS_TIMER_SET => {
            let timer_handle = HandleId::from_raw_isize(a0)?;
            let duration_ns = a1 as u64;
            timer_set(current, timer_handle, duration_ns)?;
            Ok(SyscallResult::Done(RetVal::new(0)))
        }
        SYS_TIMER_NOW => {
            let now = crate::timer::now();
            Ok(SyscallResult::Done(now.into()))
        }
        _ => {
            debug_warn!("unknown syscall: {}", n);
            Err(ErrorCode::InvalidSyscall)
        }
    }
}

pub fn syscall_handler(
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    a5: isize,
    n: isize,
) -> ! {
    let current = current_thread();
    let new_state = match do_syscall(a0, a1, a2, a3, a4, a5, n, &current) {
        Ok(SyscallResult::Done(value)) => ThreadState::Runnable(Some(value)),
        Ok(SyscallResult::Err(err)) => ThreadState::Runnable(Some(err.into())),
        Ok(SyscallResult::Block(state)) => state,
        Err(err) => ThreadState::Runnable(Some(err.into())),
    };
    current.set_state(new_state);
    drop(current);

    switch_thread();
}
