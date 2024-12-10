use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
pub use starina_types::syscall::SyscallNumber;

use crate::arch::syscall;

pub fn console_write(s: &[u8]) -> Result<(), ErrorCode> {
    let _ = syscall(
        SyscallNumber::ConsoleWrite,
        s.as_ptr() as usize,
        s.len(),
        0,
        0,
        0,
    )?;
    Ok(())
}

pub fn handle_close(handle: HandleId) -> Result<(), ErrorCode> {
    let _ = syscall(
        SyscallNumber::HandleClose,
        handle.as_i32() as usize,
        0,
        0,
        0,
        0,
    )?;
    Ok(())
}
