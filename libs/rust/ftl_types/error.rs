#[derive(Debug, PartialEq, Eq)]
#[repr(isize)]
pub enum FtlError {
    UnknownSyscall,
    TooManyHandles,
    HandleNotFound,
    HandleNotMovable,
    UnexpectedHandleType,
    InvalidSyscallReturnValue,
    NoPeer,
    InvalidArg,
    TooLarge,
    NotSupported,
}
