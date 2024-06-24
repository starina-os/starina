#[derive(Debug, PartialEq, Eq)]
#[repr(isize)]
pub enum FtlError {
    UnknownSyscall,
    TooManyHandles,
    HandleNotFound,
    UnexpectedHandleType,
    InvalidSyscallReturnValue,
    NoPeer,
}
