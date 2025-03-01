#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(isize)]
pub enum ErrorCode {
    NotAllowed,
    NotFound,
    UnexpectedType,
    AlreadyExists,
    TooManyHandles,
    HandleNotMovable,
    NoPeer,
    Empty,
    Full,
}
