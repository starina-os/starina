pub const MESSAGE_DATA_MAX_LEN: usize = 0xfff;
pub const MESSAGE_HANDLES_MAX_COUNT: usize = 3;

/// The message metadata.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct MessageInfo(isize);

impl MessageInfo {
    pub const fn from_raw(raw: isize) -> Self {
        Self(raw)
    }

    pub const fn as_raw(self) -> isize {
        self.0
    }

    pub const fn message_id(self) -> isize {
        self.0 >> 14
    }

    pub const fn num_handles(self) -> usize {
        self.0 as usize >> 12 & 0b11
    }

    pub const fn data_len(self) -> usize {
        self.0 as usize & 0xfff
    }
}
