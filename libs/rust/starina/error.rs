#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    TooManyHandles,
    HandleNotMovable,
    NoPeer,
    Empty,
}
