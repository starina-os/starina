use starina_types::address::GPAddr;
use starina_types::address::PAddr;
use starina_types::address::VAddr;
use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
use starina_types::interrupt::IrqMatcher;
use starina_types::message::MessageInfo;
use starina_types::poll::Readiness;
pub use starina_types::syscall::*;
use starina_types::timer::MonotonicTime;
use starina_types::vcpu::VCpuRunState;
use starina_types::vmspace::PageProtect;

fn syscall(
    n: u8,
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    a5: isize,
) -> Result<RetVal, ErrorCode> {
    if cfg!(feature = "in-kernel") {
        unsafe extern "C" {
            fn inkernel_syscall_entry(
                _a0: isize,
                _a1: isize,
                _a2: isize,
                _a3: isize,
                _a4: isize,
                _a5: isize,
                _n: isize,
            ) -> RetVal;
        }

        unsafe {
            let ret = inkernel_syscall_entry(a0, a1, a2, a3, a4, a5, n as isize);
            if ret.as_isize() < 0 {
                Err(ErrorCode::from(ret.as_isize()))
            } else {
                Ok(ret)
            }
        }
    } else {
        unimplemented!()
    }
}

pub fn log_write(s: &[u8]) {
    let _ = syscall(
        SYS_LOG_WRITE,
        s.as_ptr() as isize,
        s.len().try_into().unwrap(),
        0,
        0,
        0,
        0,
    );
}

pub fn thread_spawn(process: HandleId, entry: usize, arg: usize) -> Result<HandleId, ErrorCode> {
    let ret = syscall(
        SYS_THREAD_SPAWN,
        process.as_raw() as isize,
        entry.try_into().unwrap(),
        arg.try_into().unwrap(),
        0,
        0,
        0,
    )?;
    // SAFETY: The syscall returns a valid handle ID.
    let id = unsafe { HandleId::from_raw_isize(ret.as_isize()).unwrap_unchecked() };
    Ok(id)
}

pub fn thread_exit() -> ! {
    let _ = syscall(SYS_THREAD_EXIT, 0, 0, 0, 0, 0, 0);
    unreachable!("thread_exit returned");
}

pub fn poll_create() -> Result<HandleId, ErrorCode> {
    let ret = syscall(SYS_POLL_CREATE, 0, 0, 0, 0, 0, 0)?;
    // SAFETY: The syscall returns a valid handle ID.
    let id = unsafe { HandleId::from_raw_isize(ret.as_isize()).unwrap_unchecked() };
    Ok(id)
}

pub fn poll_add(poll: HandleId, object: HandleId, interests: Readiness) -> Result<(), ErrorCode> {
    syscall(
        SYS_POLL_ADD,
        poll.as_raw() as isize,
        object.as_raw() as isize,
        interests.as_isize(),
        0,
        0,
        0,
    )?;
    Ok(())
}

pub fn poll_update(
    poll: HandleId,
    object: HandleId,
    or_mask: Readiness,
    and_mask: Readiness,
) -> Result<(), ErrorCode> {
    syscall(
        SYS_POLL_UPDATE,
        poll.as_raw() as isize,
        object.as_raw() as isize,
        or_mask.as_isize(),
        and_mask.as_isize(),
        0,
        0,
    )?;
    Ok(())
}

pub fn poll_remove(poll: HandleId, object: HandleId) -> Result<(), ErrorCode> {
    syscall(
        SYS_POLL_REMOVE,
        poll.as_raw() as isize,
        object.as_raw() as isize,
        0,
        0,
        0,
        0,
    )?;
    Ok(())
}

pub fn poll_wait(poll: HandleId) -> Result<(HandleId, Readiness), ErrorCode> {
    let ret = syscall(SYS_POLL_WAIT, poll.as_raw() as isize, 0, 0, 0, 0, 0)?;
    let (id, readiness) = ret.into();
    Ok((id, readiness))
}

pub fn poll_try_wait(poll: HandleId) -> Result<(HandleId, Readiness), ErrorCode> {
    let ret = syscall(SYS_POLL_TRY_WAIT, poll.as_raw() as isize, 0, 0, 0, 0, 0)?;
    let (id, readiness) = ret.into();
    Ok((id, readiness))
}

pub fn channel_create() -> Result<(HandleId, HandleId), ErrorCode> {
    let ret = syscall(SYS_CHANNEL_CREATE, 0, 0, 0, 0, 0, 0)?;
    let first: HandleId = ret.into();
    let second = HandleId::from_raw(first.as_raw() + 1);
    Ok((first, second))
}

pub fn channel_send(
    ch: HandleId,
    msginfo: MessageInfo,
    data: *const u8,
    handles: *const HandleId,
) -> Result<(), ErrorCode> {
    syscall(
        SYS_CHANNEL_SEND,
        ch.as_raw() as isize,
        msginfo.as_raw(),
        data as isize,
        handles as isize,
        0,
        0,
    )?;
    Ok(())
}

pub fn channel_recv(
    ch: HandleId,
    data: *mut u8,
    handles: *mut HandleId,
) -> Result<MessageInfo, ErrorCode> {
    let ret = syscall(
        SYS_CHANNEL_RECV,
        ch.as_raw() as isize,
        data as isize,
        handles as isize,
        0,
        0,
        0,
    )?;
    // SAFETY: The syscall returns a valid message info.
    let msginfo = unsafe { MessageInfo::from_raw_isize(ret.as_isize()).unwrap_unchecked() };
    Ok(msginfo)
}

pub fn handle_close(handle: HandleId) -> Result<(), ErrorCode> {
    syscall(SYS_HANDLE_CLOSE, handle.as_raw() as isize, 0, 0, 0, 0, 0)?;
    Ok(())
}

pub fn folio_alloc(len: usize) -> Result<HandleId, ErrorCode> {
    let ret = syscall(SYS_FOLIO_ALLOC, len.try_into().unwrap(), 0, 0, 0, 0, 0)?;
    // SAFETY: The syscall returns a valid handle ID.
    let id = unsafe { HandleId::from_raw_isize(ret.as_isize()).unwrap_unchecked() };
    Ok(id)
}

pub fn folio_pin(paddr: PAddr, len: usize) -> Result<HandleId, ErrorCode> {
    let ret = syscall(
        SYS_FOLIO_PIN,
        paddr.as_usize() as isize,
        len.try_into().unwrap(),
        0,
        0,
        0,
        0,
    )?;
    // SAFETY: The syscall returns a valid handle ID.
    let id = unsafe { HandleId::from_raw_isize(ret.as_isize()).unwrap_unchecked() };
    Ok(id)
}

pub fn folio_paddr(handle: HandleId) -> Result<PAddr, ErrorCode> {
    let ret = syscall(SYS_FOLIO_PADDR, handle.as_raw() as isize, 0, 0, 0, 0, 0)?;
    // SAFETY: The syscall returns a valid device address.
    let paddr = PAddr::new(ret.as_isize() as usize);
    Ok(paddr)
}

pub fn vmspace_map(
    handle: HandleId,
    vaddr: VAddr,
    len: usize,
    folio: HandleId,
    offset: usize,
    prot: PageProtect,
) -> Result<VAddr, ErrorCode> {
    let ret = syscall(
        SYS_VMSPACE_MAP,
        handle.as_raw() as isize,
        vaddr.as_usize() as isize,
        len.try_into().unwrap(),
        folio.as_raw() as isize,
        offset.try_into().unwrap(),
        prot.as_raw() as isize,
    )?;
    // SAFETY: The syscall returns a valid virtual address.
    let vaddr = VAddr::new(ret.as_isize() as usize);
    Ok(vaddr)
}

pub fn interrupt_create(irq_matcher: IrqMatcher) -> Result<HandleId, ErrorCode> {
    let ret = syscall(
        SYS_INTERRUPT_CREATE,
        irq_matcher.as_raw() as isize,
        0,
        0,
        0,
        0,
        0,
    )?;
    // SAFETY: The syscall returns a valid handle ID.
    let id = unsafe { HandleId::from_raw_isize(ret.as_isize()).unwrap_unchecked() };
    Ok(id)
}

pub fn interrupt_ack(handle: HandleId) -> Result<(), ErrorCode> {
    syscall(SYS_INTERRUPT_ACK, handle.as_raw() as isize, 0, 0, 0, 0, 0)?;
    Ok(())
}

pub fn sys_hvspace_create() -> Result<HandleId, ErrorCode> {
    let ret = syscall(SYS_HVSPACE_CREATE, 0, 0, 0, 0, 0, 0)?;
    let id = unsafe { HandleId::from_raw_isize(ret.as_isize()).unwrap_unchecked() };
    Ok(id)
}

pub fn sys_hvspace_map(
    handle: HandleId,
    gpaddr: GPAddr,
    folio: HandleId,
    len: usize,
    prot: PageProtect,
) -> Result<(), ErrorCode> {
    syscall(
        SYS_HVSPACE_MAP,
        handle.as_raw() as isize,
        gpaddr.as_usize() as isize,
        folio.as_raw() as isize,
        len.try_into().unwrap(),
        prot.as_raw() as isize,
        0,
    )?;
    Ok(())
}

pub fn sys_vcpu_create(
    hvspace: HandleId,
    entry: usize,
    a0: usize,
    a1: usize,
) -> Result<HandleId, ErrorCode> {
    let ret = syscall(
        SYS_VCPU_CREATE,
        hvspace.as_raw() as isize,
        entry.try_into().unwrap(),
        a0.try_into().unwrap(),
        a1.try_into().unwrap(),
        0,
        0,
    )?;
    // SAFETY: The syscall returns a valid handle ID.
    let id = unsafe { HandleId::from_raw_isize(ret.as_isize()).unwrap_unchecked() };
    Ok(id)
}

pub fn sys_vcpu_run(vcpu: HandleId, exit: *mut VCpuRunState) -> Result<(), ErrorCode> {
    syscall(
        SYS_VCPU_RUN,
        vcpu.as_raw() as isize,
        exit as isize,
        0,
        0,
        0,
        0,
    )?;
    Ok(())
}

pub fn timer_create() -> Result<HandleId, ErrorCode> {
    let ret = syscall(SYS_TIMER_CREATE, 0, 0, 0, 0, 0, 0)?;
    let id = unsafe { HandleId::from_raw_isize(ret.as_isize()).unwrap_unchecked() };
    Ok(id)
}

pub fn timer_set(timer: HandleId, duration_ns: u64) -> Result<(), ErrorCode> {
    syscall(
        SYS_TIMER_SET,
        timer.as_raw() as isize,
        duration_ns as isize,
        0,
        0,
        0,
        0,
    )?;
    Ok(())
}

pub fn timer_now() -> Result<MonotonicTime, ErrorCode> {
    let ret = syscall(SYS_TIMER_NOW, 0, 0, 0, 0, 0, 0)?;
    Ok(MonotonicTime::from(ret))
}

pub fn log_read(buf: &mut [u8]) -> Result<usize, ErrorCode> {
    let ret = syscall(
        SYS_LOG_READ,
        buf.as_mut_ptr() as isize,
        buf.len().try_into().unwrap(),
        0,
        0,
        0,
        0,
    )?;

    let read_len = ret.as_isize() as usize;
    debug_assert!(read_len <= buf.len());
    Ok(read_len)
}
