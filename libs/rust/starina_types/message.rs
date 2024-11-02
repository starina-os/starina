use core::fmt::Debug;

use crate::handle::HandleId;

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

#[repr(C, align(16))] // Align to 16 bytes for SIMD instructions.
pub struct MessageBuffer {
    pub data: [u8; MESSAGE_DATA_MAX_LEN],
}

impl Default for MessageBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageBuffer {
    pub fn new() -> Self {
        // TODO: Avoid zeroing the buffer because it's not necessary.
        Self {
            data: [0; MESSAGE_DATA_MAX_LEN],
        }
    }

    pub fn handle_id(&self, index: usize) -> HandleId {
        debug_assert!(index < MESSAGE_HANDLES_MAX_COUNT);
        unsafe { *(self.data.as_ptr().add(index * size_of::<HandleId>()) as *const HandleId) }
    }
}

/// Invariant: `size_of::<MessageBuffer> >= size_of::<T>()`.
pub trait MessageSerialize: Debug + Sized {
    const NUM_HANDLES: usize;
    const MSGINFO: MessageInfo;
    fn serialize(self, buffer: &mut MessageBuffer);
}

pub trait MessageCallable: MessageSerialize {
    type Reply: MessageDeserialize;
}

pub trait MessageDeserialize: Debug + Sized {
    type Reader<'a>: Debug + 'a;
    fn deserialize(buffer: &mut MessageBuffer, msginfo: MessageInfo) -> Option<Self::Reader<'_>>;
}
