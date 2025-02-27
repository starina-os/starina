#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    NotFound,
    AlreadyExists,
    TooManyHandles,
    HandleNotMovable,
    NoPeer,
    Empty,
}
