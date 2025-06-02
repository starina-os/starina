use core::fmt;

macro_rules! define_errors {
    ($($name:ident = $value:expr),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(isize)]
        pub enum ErrorCode {
            $($name = $value,)*
        }

        impl From<isize> for ErrorCode {
            fn from(value: isize) -> Self {
                match value {
                    $($value => ErrorCode::$name,)*
                    _ => ErrorCode::InvalidErrorCode,
                }
            }
        }
    };
}

define_errors!(
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
    NotADevice = -21,
    AlreadyMapped = -22,
    InvalidState = -23,
    InvalidUri = -24,
    AlreadyHeld = -25,
    TooSmall = -26,
    InUse = -27
);

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
