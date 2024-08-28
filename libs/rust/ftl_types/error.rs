#[derive(Debug, PartialEq, Eq)]
#[repr(isize)]
pub enum FtlError {
    UnknownSyscall,
    TooManyHandles,
    HandleNotFound,
    HandleNotMovable,
    UnexpectedHandleType,
    NoPeer,
    InvalidArg,
    TooLarge,
    NotSupported,
    WouldBlock,
    InvalidState,
    AlreadyMapped,
}
