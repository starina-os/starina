use alloc::boxed::Box;
use core::mem;
use core::mem::MaybeUninit;
use core::mem::size_of;
use core::ops::Deref;
use core::ops::DerefMut;
use core::ptr;
use core::slice;

use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
pub use starina_types::message::*;

use crate::channel::Channel;
use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::syscall;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum MessageKind {
    Connect = 1,
    Open = 3,
    OpenReply = 4,
    StreamData = 5,
    FramedData = 6,
    Abort = 7,
    Error = 8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct CallId(u32);

impl From<u32> for CallId {
    fn from(value: u32) -> Self {
        CallId(value)
    }
}

pub struct MessageBuffer {
    data: [MaybeUninit<u8>; MESSAGE_DATA_LEN_MAX],
    handles: [HandleId; MESSAGE_NUM_HANDLES_MAX],
}

impl MessageBuffer {
    const fn new() -> Self {
        Self {
            data: [const { MaybeUninit::uninit() }; MESSAGE_DATA_LEN_MAX],
            handles: [HandleId::from_raw(0); MESSAGE_NUM_HANDLES_MAX],
        }
    }

    pub fn data_ptr(&self) -> *const u8 {
        self.data.as_ptr() as *const u8
    }

    pub fn data_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_mut_ptr() as *mut u8
    }

    pub fn handles_ptr(&self) -> *const HandleId {
        self.handles.as_ptr() as *const HandleId
    }

    pub fn handles_mut_ptr(&mut self) -> *mut HandleId {
        self.handles.as_mut_ptr() as *mut HandleId
    }
}

/// A helper to serialize the data part of a message.
struct DataWriter<'a> {
    data: &'a mut [MaybeUninit<u8>; MESSAGE_DATA_LEN_MAX],
    offset: usize,
}

impl<'a> DataWriter<'a> {
    fn new(data: &'a mut [MaybeUninit<u8>; MESSAGE_DATA_LEN_MAX]) -> Self {
        DataWriter { data, offset: 0 }
    }

    fn header_only<T: Copy>(self, header: T) -> u16 {
        let len = size_of::<T>();

        debug_assert!(len <= self.data.len());
        debug_assert_eq!(self.offset, 0);

        let data_ptr = self.data.as_mut_ptr();
        unsafe { ptr::write(data_ptr as *mut T, header) };

        len as u16
    }

    fn header_then_bytes<H: Copy>(self, header: H, bytes: &[u8]) -> u16 {
        let len = size_of::<H>() + bytes.len();

        debug_assert!(len <= self.data.len());
        debug_assert_eq!(self.offset, 0);

        let data_ptr = self.data.as_mut_ptr();
        unsafe {
            ptr::write(data_ptr as *mut H, header);
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                (data_ptr as *mut u8).add(size_of::<H>()),
                bytes.len(),
            );
        }

        len as u16
    }

    fn bytes_only(self, bytes: &[u8]) -> u16 {
        let len = bytes.len();

        debug_assert!(len <= self.data.len());
        debug_assert_eq!(self.offset, 0);

        let data_ptr = self.data.as_mut_ptr();
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), data_ptr as *mut u8, len);
        }

        len as u16
    }
}

/// A helper to serialize handles into a message.
struct HandlesWriter<'a> {
    handles: &'a mut [HandleId; MESSAGE_NUM_HANDLES_MAX],
}

impl<'a> HandlesWriter<'a> {
    fn new(handles: &'a mut [HandleId; MESSAGE_NUM_HANDLES_MAX]) -> Self {
        HandlesWriter { handles }
    }

    fn write<H: Handleable>(&mut self, index: usize, handle: H) {
        let handle_id = handle.handle_id();

        // Avoid dropping the handle; it will be moved to the peer channel.
        mem::forget(handle);

        self.handles[index] = handle_id;
    }
}

/// A helper to deserialize a message.
struct DataReader<'a> {
    data: &'a [MaybeUninit<u8>; MESSAGE_DATA_LEN_MAX],
}

impl<'a> DataReader<'a> {
    fn new(data: &'a [MaybeUninit<u8>; MESSAGE_DATA_LEN_MAX]) -> Self {
        DataReader { data }
    }

    fn header_only<T: Copy>(self, msginfo: MessageInfo) -> &'a T {
        debug_assert_eq!(size_of::<T>(), msginfo.data_len() as usize);

        let data_ptr = self.data.as_ptr() as *const T;
        let data = unsafe { &*data_ptr };
        data
    }

    fn header_then_bytes<H: Copy>(self, msginfo: MessageInfo) -> Option<(&'a H, &'a [u8])> {
        let bytes_len = msginfo.data_len().checked_sub(size_of::<H>())?;
        let data_ptr = self.data.as_ptr() as *const u8;
        let header = unsafe { &*(data_ptr as *const H) };
        let bytes = unsafe {
            let bytes_ptr = data_ptr.add(size_of::<H>());
            slice::from_raw_parts(bytes_ptr, bytes_len)
        };
        Some((header, bytes))
    }

    fn bytes_only(self, msginfo: MessageInfo) -> Option<&'a [u8]> {
        let bytes_len = msginfo.data_len();
        let data_ptr = self.data.as_ptr() as *const u8;
        let bytes = unsafe { slice::from_raw_parts(data_ptr, bytes_len) };
        Some(bytes)
    }
}

/// A helper to deserialize handles from a message.
struct HandlesReader<'a> {
    handles: &'a mut [HandleId; MESSAGE_NUM_HANDLES_MAX],
}

impl<'a> HandlesReader<'a> {
    fn new(handles: &'a mut [HandleId; MESSAGE_NUM_HANDLES_MAX]) -> Self {
        HandlesReader { handles }
    }

    fn as_channel(&mut self, msginfo: MessageInfo, index: usize) -> Channel {
        debug_assert!(index < msginfo.num_handles() as usize);

        let handle_id = self.handles[index];

        // Mark as moved.
        self.handles[index] = HandleId::from_raw(0);

        let handle = OwnedHandle::from_raw(handle_id);
        let channel = Channel::from_handle(handle);
        channel
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct RawAbortMsg {
    call_id: CallId,
    reason: ErrorCode,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct RawErrorMsg {
    reason: ErrorCode,
}

pub struct OwnedMessageBuffer(Box<MessageBuffer>);

impl OwnedMessageBuffer {
    pub fn alloc() -> Self {
        // TODO: Have a thread-local buffer pool.
        // TODO: Use `MaybeUninit` to unnecesarily zero-fill the buffer.
        let buffer = Box::new(MessageBuffer::new());
        OwnedMessageBuffer(buffer)
    }

    pub fn forget_handles(mut self) {
        self.0.handles.fill(HandleId::from_raw(0));
    }
}

impl Deref for OwnedMessageBuffer {
    type Target = MessageBuffer;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OwnedMessageBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for OwnedMessageBuffer {
    fn drop(&mut self) {
        // Drop handles.
        for handle_id in self.0.handles {
            if handle_id.as_raw() != 0 {
                if let Err(e) = syscall::handle_close(handle_id) {
                    debug_warn!("failed to close handle: {:?}", e);
                }
            }
        }
    }
}

pub enum Message<'a> {
    Connect { handle: Channel },
    Open { call_id: CallId, uri: &'a [u8] },
    OpenReply { call_id: CallId, handle: Channel },
    FramedData { data: &'a [u8] },
    StreamData { data: &'a [u8] },
    Abort { call_id: CallId, reason: ErrorCode },
    Error { reason: ErrorCode },
}

impl<'a> Message<'a> {
    pub fn serialize(self, buffer: &mut MessageBuffer) -> Result<MessageInfo, ErrorCode> {
        let data = DataWriter::new(&mut buffer.data);
        let mut handles = HandlesWriter::new(&mut buffer.handles);

        match self {
            Message::Connect { handle } => {
                handles.write(0, handle);
                Ok(MessageInfo::new(MessageKind::Connect as i32, 0, 1))
            }
            Message::Open { call_id, uri } => {
                let len = data.header_then_bytes(call_id, uri);
                Ok(MessageInfo::new(MessageKind::Open as i32, len, 0))
            }
            Message::OpenReply { call_id, handle } => {
                let len = data.header_only(call_id);
                handles.write(0, handle);
                Ok(MessageInfo::new(MessageKind::OpenReply as i32, len, 1))
            }
            Message::FramedData { data: msg_data } => {
                let len = data.bytes_only(msg_data);
                Ok(MessageInfo::new(MessageKind::FramedData as i32, len, 0))
            }
            Message::StreamData { data: msg_data } => {
                let len = data.bytes_only(msg_data);
                Ok(MessageInfo::new(MessageKind::StreamData as i32, len, 0))
            }
            Message::Abort { call_id, reason } => {
                let len = data.header_only(RawAbortMsg { call_id, reason });
                Ok(MessageInfo::new(MessageKind::Abort as i32, len, 0))
            }
            Message::Error { reason } => {
                let len = data.header_only(RawErrorMsg { reason });
                Ok(MessageInfo::new(MessageKind::Error as i32, len, 0))
            }
        }
    }

    pub fn deserialize(msginfo: MessageInfo, buffer: &'a mut MessageBuffer) -> Option<Self> {
        let data = DataReader::new(&buffer.data);
        let mut handles = HandlesReader::new(&mut buffer.handles);

        match msginfo.kind() {
            kind if kind == MessageKind::Connect as usize => {
                if msginfo.num_handles() != 1 {
                    debug_warn!(
                        "invalid number of handles for Connect message: {}",
                        msginfo.num_handles()
                    );
                    return None;
                }

                let handle = handles.as_channel(msginfo, 0);
                Some(Message::Connect { handle })
            }
            kind if kind == MessageKind::Open as usize => {
                let (call_id, uri) = data.header_then_bytes(msginfo)?;
                Some(Message::Open {
                    call_id: *call_id,
                    uri,
                })
            }
            kind if kind == MessageKind::OpenReply as usize => {
                if msginfo.num_handles() != 1 {
                    debug_warn!(
                        "invalid number of handles for OpenReply message: {}",
                        msginfo.num_handles()
                    );
                    return None;
                }

                let call_id = data.header_only(msginfo);
                let handle = handles.as_channel(msginfo, 0);
                Some(Message::OpenReply {
                    call_id: *call_id,
                    handle,
                })
            }
            kind if kind == MessageKind::FramedData as usize => {
                let data = data.bytes_only(msginfo)?;
                Some(Message::FramedData { data })
            }
            kind if kind == MessageKind::StreamData as usize => {
                let data = data.bytes_only(msginfo)?;
                Some(Message::StreamData { data })
            }
            kind if kind == MessageKind::Abort as usize => {
                let raw: &RawAbortMsg = data.header_only(msginfo);
                Some(Message::Abort {
                    call_id: raw.call_id,
                    reason: raw.reason,
                })
            }
            kind if kind == MessageKind::Error as usize => {
                let raw: &RawErrorMsg = data.header_only(msginfo);
                Some(Message::Error { reason: raw.reason })
            }
            _ => None,
        }
    }
}

pub struct OwnedMessage {
    pub msginfo: MessageInfo,
    pub buffer: OwnedMessageBuffer,
}

impl OwnedMessage {
    pub fn new(buffer: OwnedMessageBuffer, msginfo: MessageInfo) -> Self {
        Self { buffer, msginfo }
    }

    pub fn parse(&mut self) -> Option<Message<'_>> {
        Message::deserialize(self.msginfo, &mut self.buffer)
    }
}
