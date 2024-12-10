#[repr(C)]
pub struct VsyscallPage {
    pub entry: *const fn(SyscallNumber, usize, usize, usize, usize, usize) -> usize,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SyscallNumber {
    ConsoleWrite,
    ChannelSend,
    ChannelRecv,
    HandleClose,
}
