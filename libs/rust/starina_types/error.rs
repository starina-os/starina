#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(isize)]
pub enum ErrorCode {
    NotSupported = -1,
    NotAllowed = -2,
    NotFound = -3,
    InvalidMessageKind = -4,
    InvalidSyscall = -5,
    UnexpectedType = -6,
    AlreadyExists = -7,
    TooManyHandles = -8,
    HandleNotMovable = -9,
    NoPeer = -10,
    OutOfMemory = -11,
    Empty = -12,
    Full = -13,
    Closed = -14,
    InvalidMessage = -15,
    TooLongUri = -16,
    InvalidArg = -17,
    InvalidHandle = -18,
    InvalidErrorCode = -19,
    TooLarge = -20,
}

impl From<isize> for ErrorCode {
    fn from(value: isize) -> Self {
        if -19 <= value && value < 0 {
            unsafe { core::mem::transmute(value) }
        } else {
            ErrorCode::InvalidErrorCode
        }
    }
}
