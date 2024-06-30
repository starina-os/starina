use crate::error::FtlError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(isize)]
pub enum SyscallNumber {
    Print = 1,
    ChannelCreate = 2,
    ChannelSend = 3,
    ChannelRecv = 4,
    ChannelCall = 5,
    HandleClose = 6,
    PollCreate = 7,
    PollWait = 8,
    PollAdd = 9,
    FolioCreate = 10,
}

#[repr(C)]
pub struct VsyscallPage {
    pub entry: fn(isize, isize, isize, isize, isize, isize, isize) -> Result<isize, FtlError>,
    pub environ_ptr: *const u8,
    pub environ_len: usize,
}

unsafe impl Sync for VsyscallPage {}
unsafe impl Send for VsyscallPage {}
