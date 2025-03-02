#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(isize)]
pub enum ErrorCode {
    NotSupported,
    NotAllowed,
    NotFound,
    UnexpectedType,
    AlreadyExists,
    TooManyHandles,
    HandleNotMovable,
    NoPeer,
    OutOfMemory,
    Empty,
    Full,
}
