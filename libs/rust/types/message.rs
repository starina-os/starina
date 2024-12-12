use core::mem::MaybeUninit;

use crate::error::ErrorCode;


#[repr(C)]
#[derive(Debug)]
pub struct PingRaw {
    value0: u32,
    value1: u32,
    value2: u32,
    value3: u32,
    value4: u32,
    value5: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct Ping {
    value0: u32,
    value1: u32,
    value2: u32,
    value3: u32,
    value4: u32,
    value5: u32,
}

#[derive(Debug)]
pub enum Message<'a> {
    /// A message that contains a string.
    String(&'a str),
    /// A message that contains a byte array.
    Ping(Ping),
}

impl<'a> Deserialize for Message<'a> {
    fn deserialize(msginfo: MessageInfo, msgbuffer: &MessageBuffer) -> Result<Self, ErrorCode> {
        let m = match msginfo.all() {
            0x7890000 => {
                let raw = msgbuffer.data.as_ptr() as *const PingRaw;
                Message::Ping(Ping {
                    value0: unsafe { (*raw).value0 },
                    value1: unsafe { (*raw).value1 },
                    value2: unsafe { (*raw).value2 },
                    value3: unsafe { (*raw).value3 },
                    value4: unsafe { (*raw).value4 },
                    value5: unsafe { (*raw).value5 },
                })
            }
            _ => { return Err(ErrorCode::Foo)},
        };

        Ok(m)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct MessageInfo(u32);

impl MessageInfo {
    pub const fn from_raw(raw: u32) -> MessageInfo {
        MessageInfo(raw)
    }

    const fn new(id: u32, len: u16, num_handles: u8) -> MessageInfo {
        debug_assert!(id < 1 << 16);
        debug_assert!(len < 1 << 14);
        debug_assert!(num_handles < 1 << 2);
        MessageInfo(id << 16 | (num_handles as u32) << 14 | (len as u32))
    }

    fn all(self) -> u32 {
        self.0
    }

    fn id_and_num_handles_bits(self) -> u32 {
        (self.0 & 0xffff_c000)
    }

    fn len(self) -> u16 {
        (self.0 & 0x3fff) as u16
    }
}

#[repr(C)]
pub struct MessageBuffer {
    pub data: [u8; 4095],
    _pad: u8,
}

impl MessageBuffer {
    pub const fn new() -> MessageBuffer {
        // SAFETY: The buffer will be initialized by IPC stubs when sending,
        //         a message, and by the kernel when receiving a message.
        unsafe {
            // Use MaybeUninit to memset the buffer to zero unnecessarily and
            // just prepare an uninitialized buffer on the stack.
            let uninit = MaybeUninit::<MessageBuffer>::uninit();
            uninit.assume_init()
        }
    }
}

pub trait Deserialize: Sized {
    fn deserialize(msginfo: MessageInfo, msgbuffer: &MessageBuffer) -> Result<Self, ErrorCode>;
}

pub trait Serialize {
    fn serialize(self, msgbuffer: &mut MessageBuffer) -> Result<MessageInfo, ErrorCode>;
}
