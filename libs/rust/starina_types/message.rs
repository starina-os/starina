use crate::error::ErrorCode;
use crate::handle::HandleId;
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

pub struct MessageBuffer {
    pub data: [u8; MESSAGE_DATA_LEN_MAX],
    pub handles: [HandleId; MESSAGE_NUM_HANDLES_MAX],
}

impl MessageBuffer {
    pub const unsafe fn data_as_ref<T>(&self) -> &T {
        debug_assert!(size_of::<T>() <= MESSAGE_DATA_LEN_MAX);
        unsafe { &*(self.data.as_ptr() as *const T) }
    }

    pub const unsafe fn data_as_mut<T>(&mut self) -> &mut T {
        debug_assert!(size_of::<T>() <= MESSAGE_DATA_LEN_MAX);
        unsafe { &mut *(self.data.as_mut_ptr() as *mut T) }
    }
}

#[repr(u8)]
pub enum MessageKind {
    Connect = 1,
    Open = 3,
    OpenReply = 4,
    StreamData = 5,
    FramedData = 6,
}

pub trait Messageable {
    type This<'a>;
    fn kind() -> MessageKind;
    fn write(self, buffer: &mut MessageBuffer) -> Result<MessageInfo, ErrorCode>;
    /// # Safety
    ///
    /// This method does not check the message kind. It's caller's
    /// responsibility to ensure the message kind is correct.
    unsafe fn is_valid(msginfo: MessageInfo, buffer: &MessageBuffer) -> bool;
    /// # Safety
    ///
    /// This method does not check the validity of the message. It's caller's
    /// responsibility to make sure:
    ///
    /// - The message kind is correct.
    /// - The validity of the message is checked by `is_valid`.
    unsafe fn cast_unchecked(msginfo: MessageInfo, buffer: &MessageBuffer) -> Self::This<'_>;
}

pub const URI_LEN_MAX: usize = 1024;

pub struct Connect {
    handle: HandleId,
}

impl Messageable for Connect {
    type This<'a> = Connect;

    fn kind() -> MessageKind {
        MessageKind::Connect
    }

    unsafe fn is_valid(msginfo: MessageInfo, _buffer: &MessageBuffer) -> bool {
        msginfo.data_len() == 0 && msginfo.num_handles() == 1
    }

    unsafe fn cast_unchecked(msginfo: MessageInfo, buffer: &MessageBuffer) -> Self::This<'_> {
        Connect {
            handle: buffer.handles[0],
        }
    }

    fn write(self, buffer: &mut MessageBuffer) -> Result<MessageInfo, ErrorCode> {
        buffer.handles[0] = self.handle;
        Ok(MessageInfo::new(MessageKind::Connect as i32, 0, 1))
    }
}

pub struct RawOpen {
    pub uri: [u8; URI_LEN_MAX],
}

pub struct Open<'a> {
    pub uri: &'a str,
}

impl Messageable for Open<'_> {
    type This<'a> = Open<'a>;

    fn kind() -> MessageKind {
        MessageKind::Open
    }

    unsafe fn is_valid(msginfo: MessageInfo, buffer: &MessageBuffer) -> bool {
        if msginfo.data_len() > URI_LEN_MAX {
            return false;
        }

        let raw = unsafe { buffer.data_as_ref::<RawOpen>() };
        if core::str::from_utf8(&raw.uri[..msginfo.data_len()]).is_err() {
            return false;
        }

        true
    }

    unsafe fn cast_unchecked(msginfo: MessageInfo, buffer: &MessageBuffer) -> Self::This<'_> {
        unsafe {
            let raw = buffer.data_as_ref::<RawOpen>();
            let uri = core::str::from_utf8_unchecked(&raw.uri[..msginfo.data_len()]);
            Open { uri }
        }
    }

    fn write(self, buffer: &mut MessageBuffer) -> Result<MessageInfo, ErrorCode> {
        let raw = unsafe { buffer.data_as_mut::<RawOpen>() };
        if self.uri.len() > URI_LEN_MAX {
            return Err(ErrorCode::TooLongUri);
        }

        raw.uri[..self.uri.len()].copy_from_slice(self.uri.as_bytes());
        Ok(MessageInfo::new(
            MessageKind::Open as i32,
            self.uri.len() as u16,
            0,
        ))
    }
}

pub struct FramedData<'a> {
    pub data: &'a [u8],
}

impl Messageable for FramedData<'_> {
    type This<'a> = FramedData<'a>;

    fn kind() -> MessageKind {
        MessageKind::FramedData
    }

    unsafe fn is_valid(msginfo: MessageInfo, buffer: &MessageBuffer) -> bool {
        msginfo.data_len() <= buffer.data.len()
    }

    unsafe fn cast_unchecked(msginfo: MessageInfo, buffer: &MessageBuffer) -> Self::This<'_> {
        unsafe {
            let data = &buffer.data[..msginfo.data_len()];
            FramedData { data }
        }
    }

    fn write(self, buffer: &mut MessageBuffer) -> Result<MessageInfo, ErrorCode> {
        if self.data.len() > buffer.data.len() {
            return Err(ErrorCode::TooLarge);
        }

        buffer.data[..self.data.len()].copy_from_slice(self.data);
        Ok(MessageInfo::new(
            MessageKind::FramedData as i32,
            self.data.len() as u16,
            0,
        ))
    }
}
