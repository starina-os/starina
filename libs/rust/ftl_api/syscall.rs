use ftl_types::address::PAddr;
use ftl_types::address::VAddr;
use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::interrupt::Irq;
use ftl_types::message::MessageBuffer;
use ftl_types::message::MessageInfo;
use ftl_types::poll::PollEvent;
use ftl_types::poll::PollSyscallResult;
use ftl_types::signal::SignalBits;
use ftl_types::syscall::SyscallNumber;
use ftl_types::syscall::VsyscallPage;
use ftl_types::vmspace::PageProtect;
use spin::Mutex;

static VSYSCALL_PAGE: Mutex<Option<&'static VsyscallPage>> = Mutex::new(None);

pub fn syscall(
    n: SyscallNumber,
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    a5: isize,
) -> Result<isize, FtlError> {
    let vsyscall = VSYSCALL_PAGE.lock().expect("vsyscall not set");
    (vsyscall.entry)(n as isize, a0, a1, a2, a3, a4, a5)
}

pub fn syscall0(n: SyscallNumber) -> Result<isize, FtlError> {
    syscall(n, 0, 0, 0, 0, 0, 0)
}

pub fn syscall1(n: SyscallNumber, a0: isize) -> Result<isize, FtlError> {
    syscall(n, a0, 0, 0, 0, 0, 0)
}

pub fn syscall2(n: SyscallNumber, a0: isize, a1: isize) -> Result<isize, FtlError> {
    syscall(n, a0, a1, 0, 0, 0, 0)
}

pub fn syscall3(n: SyscallNumber, a0: isize, a1: isize, a2: isize) -> Result<isize, FtlError> {
    syscall(n, a0, a1, a2, 0, 0, 0)
}

pub fn syscall4(
    n: SyscallNumber,
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
) -> Result<isize, FtlError> {
    syscall(n, a0, a1, a2, a3, 0, 0)
}

pub fn syscall5(
    n: SyscallNumber,
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
) -> Result<isize, FtlError> {
    syscall(n, a0, a1, a2, a3, a4, 0)
}

pub fn syscall6(
    n: SyscallNumber,
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    a5: isize,
) -> Result<isize, FtlError> {
    syscall(n, a0, a1, a2, a3, a4, a5)
}

pub fn handle_close(handle: HandleId) -> Result<(), FtlError> {
    syscall1(SyscallNumber::HandleClose, handle.as_isize())?;
    Ok(())
}

pub fn print(s: &[u8]) -> Result<(), FtlError> {
    syscall2(SyscallNumber::Print, s.as_ptr() as isize, s.len() as isize)?;
    Ok(())
}

pub fn folio_create(len: usize) -> Result<HandleId, FtlError> {
    let ret = syscall1(SyscallNumber::FolioCreate, len as isize)?;
    let handle_id = HandleId::from_raw_isize_truncated(ret);
    Ok(handle_id)
}

pub fn folio_create_fixed(paddr: PAddr, len: usize) -> Result<HandleId, FtlError> {
    let ret = syscall2(
        SyscallNumber::FolioCreateFixed,
        paddr.as_usize() as isize,
        len as isize,
    )?;
    let handle_id = HandleId::from_raw_isize_truncated(ret);
    Ok(handle_id)
}

pub fn folio_paddr(handle: HandleId) -> Result<usize, FtlError> {
    let ret = syscall1(SyscallNumber::FolioPAddr, handle.as_isize())?;
    Ok(ret as usize)
}

pub fn vmspace_map(
    handle: HandleId,
    len: usize,
    folio: HandleId,
    prot: PageProtect,
) -> Result<VAddr, FtlError> {
    let ret = syscall4(
        SyscallNumber::VmSpaceMap,
        handle.as_isize(),
        len as isize,
        folio.as_isize(),
        prot.as_raw() as isize,
    )?;

    Ok(VAddr::new(ret as usize))
}

pub fn poll_create() -> Result<HandleId, FtlError> {
    let ret = syscall0(SyscallNumber::PollCreate)?;
    let handle_id = HandleId::from_raw_isize_truncated(ret);
    Ok(handle_id)
}

pub fn poll_add(
    poll_handle_id: HandleId,
    target_handle_id: HandleId,
    interests: PollEvent,
) -> Result<(), FtlError> {
    syscall3(
        SyscallNumber::PollAdd,
        poll_handle_id.as_isize(),
        target_handle_id.as_isize(),
        interests.as_raw() as isize,
    )?;
    Ok(())
}

pub fn poll_wait(handle: HandleId) -> Result<PollSyscallResult, FtlError> {
    let ret = syscall1(SyscallNumber::PollWait, handle.as_isize())?;
    Ok(PollSyscallResult::from_raw(ret))
}

pub fn channel_create() -> Result<(HandleId, HandleId), FtlError> {
    let ret = syscall0(SyscallNumber::ChannelCreate)?;
    let handle0 = HandleId::from_raw_isize_truncated(ret);
    let handle1 = HandleId::from_raw_isize_truncated(ret + 1);
    Ok((handle0, handle1))
}

pub fn channel_send(
    handle: HandleId,
    msginfo: MessageInfo,
    msgbuffer: *const MessageBuffer,
) -> Result<(), FtlError> {
    syscall3(
        SyscallNumber::ChannelSend,
        handle.as_isize(),
        msginfo.as_raw(),
        msgbuffer as isize,
    )?;
    Ok(())
}

pub fn channel_recv(
    handle: HandleId,
    msgbuffer: *mut MessageBuffer,
) -> Result<MessageInfo, FtlError> {
    let ret = syscall2(
        SyscallNumber::ChannelRecv,
        handle.as_isize(),
        msgbuffer as isize,
    )?;
    Ok(MessageInfo::from_raw(ret))
}

pub fn signal_create() -> Result<HandleId, FtlError> {
    let ret = syscall0(SyscallNumber::SignalCreate)?;
    let handle_id = HandleId::from_raw_isize_truncated(ret);
    Ok(handle_id)
}

pub fn signal_update(handle: HandleId, value: SignalBits) -> Result<(), FtlError> {
    syscall2(
        SyscallNumber::SignalUpdate,
        handle.as_isize(),
        value.as_i32() as isize,
    )?;
    Ok(())
}

pub fn signal_clear(handle: HandleId) -> Result<SignalBits, FtlError> {
    let ret = syscall1(SyscallNumber::SignalClear, handle.as_isize())?;
    Ok(SignalBits::from_raw(ret as i32))
}

pub fn interrupt_create(irq: Irq) -> Result<HandleId, FtlError> {
    let ret = syscall1(SyscallNumber::InterruptCreate, irq.as_usize() as isize)?;
    let handle_id = HandleId::from_raw_isize_truncated(ret);
    Ok(handle_id)
}

pub fn interrupt_ack(handle: HandleId) -> Result<(), FtlError> {
    syscall1(SyscallNumber::InterruptAck, handle.as_isize())?;
    Ok(())
}

pub(crate) fn set_vsyscall(vsyscall: &'static VsyscallPage) {
    *VSYSCALL_PAGE.lock() = Some(vsyscall);
}
