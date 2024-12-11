use starina_types::error::ErrorCode;
use starina_types::handle::{HandleId, HandleIdWithBits};
pub use starina_types::syscall::SyscallNumber;

use crate::arch;

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
    let _ = syscall2(
        SyscallNumber::ConsoleWrite,
        s.as_ptr() as usize,
        s.len(),
    )?;
    Ok(())
}

/// Closes a handle.
pub fn handle_close(handle: HandleId) -> Result<(), ErrorCode> {
    let _ = syscall1(
        SyscallNumber::HandleClose,
        handle.as_i32() as usize,




    )?;
    Ok(())
}


/// Waits for an event on a poll object. Blocking.
pub fn poll_wait(poll: HandleId) -> Result<HandleIdWithBits, ErrorCode> {
    let ret = syscall1(SyscallNumber::PollWait, poll.as_i32() as usize)?;
    Ok(HandleIdWithBits::from_raw(ret as i32))
}
