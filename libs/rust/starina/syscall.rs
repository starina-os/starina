use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
use starina_types::handle::HandleIdWithBits;
use starina_types::message::MessageBuffer;
use starina_types::message::MessageInfo;
pub use starina_types::syscall::SyscallNumber;

use crate::arch;

#[inline]
fn syscall0(n: SyscallNumber) -> Result<isize, ErrorCode> {
    arch::syscall(n, 0, 0, 0, 0, 0)
}

#[inline]
fn syscall1(n: SyscallNumber, arg1: usize) -> Result<isize, ErrorCode> {
    arch::syscall(n, arg1, 0, 0, 0, 0)
}

#[inline]
fn syscall2(n: SyscallNumber, arg1: usize, arg2: usize) -> Result<isize, ErrorCode> {
    arch::syscall(n, arg1, arg2, 0, 0, 0)
}

/// Write a string to the debug console.
pub fn console_write(s: &[u8]) -> Result<(), ErrorCode> {
    let _ = syscall2(SyscallNumber::ConsoleWrite, s.as_ptr() as usize, s.len())?;
    Ok(())
}

/// Closes a handle.
pub fn handle_close(handle: HandleId) -> Result<(), ErrorCode> {
    let _ = syscall1(SyscallNumber::HandleClose, handle.as_i32() as usize)?;
    Ok(())
}

/// Tries to receive a message from a channel. Non-blocking.
pub fn channel_try_recv(
    handle: HandleId,
    msgbuffer: *mut MessageBuffer,
) -> Result<MessageInfo, ErrorCode> {
    let ret = syscall2(
        SyscallNumber::ChannelTryRecv,
        handle.as_i32() as usize,
        msgbuffer as usize,
    )?;
    Ok(MessageInfo::from_raw(ret as u32))
}

/// Creates a poll object.
pub fn poll_create() -> Result<HandleId, ErrorCode> {
    let ret = syscall0(SyscallNumber::PollCreate)?;
    let handle_id = HandleId::from_raw(ret as i32); // FIXME:
    Ok(handle_id)
}

/// Waits for an event on a poll object. Blocking.
pub fn poll_wait(poll: HandleId) -> Result<HandleIdWithBits, ErrorCode> {
    let ret = syscall1(SyscallNumber::PollWait, poll.as_i32() as usize)?;
    Ok(HandleIdWithBits::from_raw(ret as i32))
}
