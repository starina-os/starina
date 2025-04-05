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
    data: [u8; MESSAGE_DATA_LEN_MAX],
    handles: [HandleId; MESSAGE_NUM_HANDLES_MAX],
}

impl MessageBuffer {
    pub fn zeroed() -> Self {
        Self {
            data: [0; MESSAGE_DATA_LEN_MAX],
            handles: [HandleId::from_raw(0); MESSAGE_NUM_HANDLES_MAX],
        }
    }

    pub const fn data(&self) -> &[u8] {
        &self.data
    }

    pub const fn handles(&self) -> &[HandleId] {
        &self.handles
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn handles_mut(&mut self) -> &mut [HandleId] {
        &mut self.handles
    }

    pub unsafe fn data_as_ref<T>(&self) -> &T {
        debug_assert!(size_of::<T>() <= MESSAGE_DATA_LEN_MAX);
        debug_assert!(self.data.as_ptr().is_aligned_to(align_of::<T>()));

        unsafe { &*(self.data.as_ptr() as *const T) }
    }

    pub unsafe fn data_as_mut<T>(&mut self) -> &mut T {
        debug_assert!(size_of::<T>() <= MESSAGE_DATA_LEN_MAX);
        debug_assert!(self.data.as_ptr().is_aligned_to(align_of::<T>()));

        unsafe { &mut *(self.data.as_mut_ptr() as *mut T) }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum MessageKind {
    Connect = 1,
    Open = 3,
    OpenReply = 4,
    StreamData = 5,
    FramedData = 6,
}

pub trait Messageable<'a> {
    fn kind() -> MessageKind;
    fn write(self, buffer: &mut MessageBuffer) -> Result<MessageInfo, ErrorCode>;
    unsafe fn parse_unchecked(msginfo: MessageInfo, buffer: &'a MessageBuffer) -> Option<Self>
    where
        Self: Sized;
}

pub const URI_LEN_MAX: usize = 1024;

pub struct Connect {
    pub handle: HandleId,
}

impl<'a> Messageable<'a> for Connect {
    fn kind() -> MessageKind {
        MessageKind::Connect
    }

    fn write(self, buffer: &mut MessageBuffer) -> Result<MessageInfo, ErrorCode> {
        buffer.handles[0] = self.handle;
        Ok(MessageInfo::new(MessageKind::Connect as i32, 0, 1))
    }

    unsafe fn parse_unchecked(msginfo: MessageInfo, buffer: &'a MessageBuffer) -> Option<Self> {
        Some(Connect {
            handle: buffer.handles[0],
        })
    }
}

pub struct RawOpen {
    pub uri: [u8; URI_LEN_MAX],
}

pub struct Open<'a> {
    pub uri: &'a str,
}

impl<'a> Messageable<'a> for Open<'a> {
    fn kind() -> MessageKind {
        MessageKind::Open
    }

    unsafe fn parse_unchecked(msginfo: MessageInfo, buffer: &'a MessageBuffer) -> Option<Self> {
        let raw = unsafe { buffer.data_as_ref::<RawOpen>() };
        let uri = unsafe { core::str::from_utf8_unchecked(&raw.uri[..msginfo.data_len()]) };
        Some(Open { uri })
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

impl<'a> Messageable<'a> for FramedData<'a> {
    fn kind() -> MessageKind {
        MessageKind::FramedData
    }

    unsafe fn parse_unchecked(msginfo: MessageInfo, buffer: &'a MessageBuffer) -> Option<Self> {
        let data = &buffer.data[..msginfo.data_len()];
        Some(FramedData { data })
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
