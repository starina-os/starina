use starina_types::address::DAddr;
use starina_types::address::VAddr;
use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
use starina_types::message::MessageInfo;
use starina_types::poll::Readiness;
pub use starina_types::syscall::*;
use starina_types::vmspace::PageProtect;

fn syscall(
    n: u8,
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
) -> Result<RetVal, ErrorCode> {
    #[cfg(feature = "in-kernel")]
    {
        unsafe extern "C" {
            fn enter_kernelland(
                _a0: isize,
                _a1: isize,
                _a2: isize,
                _a3: isize,
                _a4: isize,
                _a5: isize,
            ) -> RetVal;
        }

        unsafe {
            let ret = enter_kernelland(a0, a1, a2, a3, a4, n as isize);
            if ret.as_isize() < 0 {
                Err(ErrorCode::from(ret.as_isize()))
            } else {
                Ok(ret)
            }
        }
    }
}

pub fn console_write(s: &[u8]) {
    let _ = syscall(
        SYS_CONSOLE_WRITE,
        s.as_ptr() as isize,
        s.len().try_into().unwrap(),
        0,
        0,
        0,
    );
}

pub fn poll_create() -> Result<HandleId, ErrorCode> {
    let ret = syscall(SYS_POLL_CREATE, 0, 0, 0, 0, 0)?;
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
    )?;
    Ok(())
}

pub fn poll_wait(poll: HandleId) -> Result<(HandleId, Readiness), ErrorCode> {
    let ret = syscall(SYS_POLL_WAIT, poll.as_raw() as isize, 0, 0, 0, 0)?;
    // SAFETY: The syscall returns a valid handle ID.
    let id = unsafe { HandleId::from_raw_isize(ret.as_isize()).unwrap_unchecked() };
    // SAFETY: The syscall returns a valid readiness.
    let readiness = unsafe { Readiness::from_raw_isize(ret.as_isize()).unwrap_unchecked() };
    Ok((id, readiness))
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
    )?;
    // SAFETY: The syscall returns a valid message info.
    let msginfo = unsafe { MessageInfo::from_raw_isize(ret.as_isize()).unwrap_unchecked() };
    Ok(msginfo)
}

pub fn handle_close(handle: HandleId) -> Result<(), ErrorCode> {
    syscall(SYS_HANDLE_CLOSE, handle.as_raw() as isize, 0, 0, 0, 0)?;
    Ok(())
}

pub fn folio_create_mmio(
    iobus: HandleId,
    daddr: Option<DAddr>,
    len: usize,
) -> Result<HandleId, ErrorCode> {
    let ret = syscall(
        SYS_FOLIO_CREATE_MMIO,
        iobus.as_raw() as isize,
        daddr.map_or(0, |daddr| daddr.as_usize() as isize),
        len.try_into().unwrap(),
        0,
        0,
    )?;

    // SAFETY: The syscall returns a valid handle ID.
    let id = unsafe { HandleId::from_raw_isize(ret.as_isize()).unwrap_unchecked() };
    Ok(id)
}

pub fn folio_daddr(handle: HandleId) -> Result<DAddr, ErrorCode> {
    let ret = syscall(SYS_FOLIO_DADDR, handle.as_raw() as isize, 0, 0, 0, 0)?;
    // SAFETY: The syscall returns a valid device address.
    let daddr = DAddr::new(ret.as_isize() as usize);
    Ok(daddr)
}

pub fn vmspace_map(
    handle: HandleId,
    folio: HandleId,
    prot: PageProtect,
) -> Result<VAddr, ErrorCode> {
    let ret = syscall(
        SYS_VMSPACE_MAP,
        handle.as_raw() as isize,
        folio.as_raw() as isize,
        prot.as_raw() as isize,
        0,
        0,
    )?;
    // SAFETY: The syscall returns a valid virtual address.
    let vaddr = VAddr::new(ret.as_isize() as usize);
    Ok(vaddr)
}
