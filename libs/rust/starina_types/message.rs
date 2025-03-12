use crate::handle::HandleId;

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
