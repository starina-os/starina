use crate::error::ErrorCode;
use crate::syscall::RetVal;

pub const MESSAGE_NUM_HANDLES_MAX: usize = 3;
pub const MESSAGE_DATA_LEN_MAX: usize = 4 * 1024;

/// The message metadata.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct MessageInfo(i32);

impl MessageInfo {
    pub const fn new(kind: i32, data_len: u16, num_handles: u8) -> Self {
        debug_assert!(num_handles <= MESSAGE_NUM_HANDLES_MAX as u8);
        MessageInfo((kind << 18) | ((num_handles as i32) << 16) | (data_len as i32))
    }

    pub fn from_raw_isize(raw: isize) -> Result<Self, ErrorCode> {
        match i32::try_from(raw) {
            Ok(raw) if raw >= 0 => Ok(MessageInfo(raw)),
            _ => Err(ErrorCode::InvalidArg),
        }
    }

    pub fn as_raw(&self) -> isize {
        self.0 as isize
    }

    pub fn kind(self) -> usize {
        (self.0 >> 18) as usize
    }

    pub fn data_len(self) -> usize {
        (self.0 & 0xffff) as usize
    }

    pub fn num_handles(self) -> usize {
        ((self.0 >> 16) & 0b11) as usize
    }
}

impl From<MessageInfo> for RetVal {
    fn from(msginfo: MessageInfo) -> Self {
        RetVal::new(msginfo.0 as isize)
    }
}
