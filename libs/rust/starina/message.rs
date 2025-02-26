pub const MESSAGE_NUM_HANDLES_MAX: usize = 3;

/// The message metadata.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct MessageInfo(i32);

impl MessageInfo {
    pub const fn new(kind: i32, data_len: u16, num_handles: u8) -> Self {
        debug_assert!(num_handles <= MESSAGE_NUM_HANDLES_MAX as u8);
        MessageInfo((kind << 18) | ((num_handles as i32) << 16) | (data_len as i32))
    }

    pub fn data_len(self) -> usize {
        (self.0 & 0xffff) as usize
    }

    pub fn num_handles(self) -> usize {
        ((self.0 >> 16) & 0b11) as usize
    }
}
