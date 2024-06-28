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
    pub handles: [HandleId; MESSAGE_HANDLES_MAX_COUNT],
}

impl MessageBuffer {
    pub fn new() -> Self {
        // TODO: Avoid zeroing the buffer because it's not necessary.
        Self {
            data: [0; MESSAGE_DATA_MAX_LEN],
            handles: [HandleId::from_raw(0); MESSAGE_HANDLES_MAX_COUNT],
        }
    }
}

/// Invariant: size_of::<MessageBuffer> >= size_of::<T>().
pub trait MessageSerialize: Sized {
    const MSGINFO: MessageInfo;
    fn serialize(self, buffer: &mut MessageBuffer);
}

pub trait MessageDeserialize: Sized {
    type Reader<'a>: 'a;
    fn deserialize<'a>(buffer: &'a MessageBuffer, msginfo: MessageInfo)
        -> Option<Self::Reader<'a>>;
}

#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct HandleOwnership(pub HandleId);
