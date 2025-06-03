use alloc::boxed::Box;
use core::mem;
use core::mem::MaybeUninit;
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
pub struct CallId(pub u32);

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

pub struct DataWriter<'a> {
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

pub struct HandlesWriter<'a> {
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

pub struct DataReader<'a> {
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

pub struct HandlesReader<'a> {
    handles: &'a mut [HandleId; MESSAGE_NUM_HANDLES_MAX],
}

impl<'a> HandlesReader<'a> {
    fn new(handles: &'a mut [HandleId; MESSAGE_NUM_HANDLES_MAX]) -> Self {
        HandlesReader { handles }
    }

    fn as_channel<const I: usize>(&mut self, msginfo: MessageInfo) -> Channel {
        debug_assert!(I < msginfo.num_handles() as usize);

        let handle_id = self.handles[I];

        // Mark as moved.
        self.handles[I] = HandleId::from_raw(0);

        let handle = OwnedHandle::from_raw(handle_id);
        let channel = Channel::from_handle(handle);
        channel
    }
}

pub trait Sendable: Sized {
    fn serialize(
        self,
        data: DataWriter<'_>,
        handles: HandlesWriter<'_>,
    ) -> Result<MessageInfo, ErrorCode>;

    fn serialize_to_buffer(self, buffer: &mut MessageBuffer) -> Result<MessageInfo, ErrorCode> {
        let data = DataWriter::new(&mut buffer.data);
        let handles = HandlesWriter::new(&mut buffer.handles);
        self.serialize(data, handles)
    }
}

pub trait Replyable: Sized {
    fn serialize(
        self,
        call_id: CallId,
        data: DataWriter<'_>,
        handles: HandlesWriter<'_>,
    ) -> Result<MessageInfo, ErrorCode>;

    fn serialize_to_buffer(
        self,
        call_id: CallId,
        buffer: &mut MessageBuffer,
    ) -> Result<MessageInfo, ErrorCode> {
        let data = DataWriter::new(&mut buffer.data);
        let handles = HandlesWriter::new(&mut buffer.handles);
        self.serialize(call_id, data, handles)
    }
}

pub trait Callable: Sized {
    fn serialize(
        self,
        call_id: CallId,
        data: DataWriter<'_>,
        handles: HandlesWriter<'_>,
    ) -> Result<MessageInfo, ErrorCode>;

    fn serialize_to_buffer(
        self,
        call_id: CallId,
        buffer: &mut MessageBuffer,
    ) -> Result<MessageInfo, ErrorCode> {
        let data = DataWriter::new(&mut buffer.data);
        let handles = HandlesWriter::new(&mut buffer.handles);
        self.serialize(call_id, data, handles)
    }
}

pub trait Receivable<'a>: Sized {
    fn deserialize(
        msginfo: MessageInfo,
        data: DataReader<'a>,
        handles: HandlesReader<'a>,
    ) -> Option<Self>;

    fn deserialize_from_buffer(
        msginfo: MessageInfo,
        buffer: &'a mut MessageBuffer,
    ) -> Option<Self> {
        let data = DataReader::new(&buffer.data);
        let handles = HandlesReader::new(&mut buffer.handles);
        Self::deserialize(msginfo, data, handles)
    }
}

pub struct OpenMsg<'a> {
    pub uri: &'a [u8],
}

impl Callable for OpenMsg<'_> {
    fn serialize(
        self,
        call_id: CallId,
        data: DataWriter<'_>,
        _handles: HandlesWriter<'_>,
    ) -> Result<MessageInfo, ErrorCode> {
        let len = data.header_then_bytes(call_id, self.uri);
        Ok(MessageInfo::new(MessageKind::Open as i32, len, 0))
    }
}

impl<'a> Receivable<'a> for (CallId, OpenMsg<'a>) {
    fn deserialize(
        msginfo: MessageInfo,
        data: DataReader<'a>,
        _handles: HandlesReader<'a>,
    ) -> Option<Self> {
        let (call_id, uri) = data.header_then_bytes(msginfo)?;
        Some((*call_id, OpenMsg { uri }))
    }
}

pub struct OpenReplyMsg {
    pub handle: Channel,
}

impl<'a> Receivable<'a> for (CallId, OpenReplyMsg) {
    fn deserialize(
        msginfo: MessageInfo,
        data: DataReader<'a>,
        mut handles: HandlesReader<'a>,
    ) -> Option<Self> {
        let call_id = data.header_only(msginfo);
        let handle = handles.as_channel::<0>(msginfo);
        Some((*call_id, OpenReplyMsg { handle }))
    }
}

impl Replyable for OpenReplyMsg {
    fn serialize(
        self,
        call_id: CallId,
        data: DataWriter<'_>,
        mut handles: HandlesWriter<'_>,
    ) -> Result<MessageInfo, ErrorCode> {
        let len = data.header_only(call_id);
        handles.write(0, self.handle);
        Ok(MessageInfo::new(MessageKind::OpenReply as i32, len, 1))
    }
}

pub struct ConnectMsg {
    pub handle: Channel,
}

impl Sendable for ConnectMsg {
    fn serialize(
        self,
        _data: DataWriter<'_>,
        mut handles: HandlesWriter<'_>,
    ) -> Result<MessageInfo, ErrorCode> {
        handles.write(0, self.handle);
        Ok(MessageInfo::new(MessageKind::Connect as i32, 0, 1))
    }
}

impl<'a> Receivable<'a> for ConnectMsg {
    fn deserialize(
        msginfo: MessageInfo,
        _data: DataReader<'a>,
        mut handles: HandlesReader<'a>,
    ) -> Option<Self> {
        let handle = handles.as_channel::<0>(msginfo);
        Some(ConnectMsg { handle })
    }
}

pub struct FramedDataMsg<'a> {
    pub data: &'a [u8],
}

impl Sendable for FramedDataMsg<'_> {
    fn serialize(
        self,
        data: DataWriter<'_>,
        _handles: HandlesWriter<'_>,
    ) -> Result<MessageInfo, ErrorCode> {
        let len = data.bytes_only(self.data);
        Ok(MessageInfo::new(MessageKind::FramedData as i32, len, 0))
    }
}

impl<'a> Receivable<'a> for FramedDataMsg<'a> {
    fn deserialize(
        msginfo: MessageInfo,
        data: DataReader<'a>,
        _handles: HandlesReader<'a>,
    ) -> Option<Self> {
        let bytes = data.bytes_only(msginfo)?;
        Some(FramedDataMsg { data: bytes })
    }
}

pub struct StreamDataMsg<'a> {
    pub data: &'a [u8],
}

impl Sendable for StreamDataMsg<'_> {
    fn serialize(
        self,
        data: DataWriter<'_>,
        _handles: HandlesWriter<'_>,
    ) -> Result<MessageInfo, ErrorCode> {
        let len = data.bytes_only(self.data);
        Ok(MessageInfo::new(MessageKind::StreamData as i32, len, 0))
    }
}

impl<'a> Receivable<'a> for StreamDataMsg<'a> {
    fn deserialize(
        msginfo: MessageInfo,
        data: DataReader<'a>,
        _handles: HandlesReader<'a>,
    ) -> Option<Self> {
        let bytes = data.bytes_only(msginfo)?;
        Some(StreamDataMsg { data: bytes })
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct RawAbort {
    call_id: CallId,
    reason: ErrorCode,
}

pub struct AbortMsg {
    pub reason: ErrorCode,
}

impl Replyable for AbortMsg {
    fn serialize(
        self,
        call_id: CallId,
        data: DataWriter<'_>,
        _handles: HandlesWriter<'_>,
    ) -> Result<MessageInfo, ErrorCode> {
        let len = data.header_only(RawAbort {
            call_id,
            reason: self.reason,
        });
        Ok(MessageInfo::new(MessageKind::Abort as i32, len as u16, 0))
    }
}

impl<'a> Receivable<'a> for (CallId, AbortMsg) {
    fn deserialize(
        msginfo: MessageInfo,
        data: DataReader<'a>,
        _handles: HandlesReader<'a>,
    ) -> Option<Self> {
        let raw: &RawAbort = data.header_only(msginfo);
        Some((raw.call_id, AbortMsg { reason: raw.reason }))
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct RawError {
    reason: ErrorCode,
}

#[derive(Debug, Clone)]
pub struct ErrorMsg {
    pub reason: ErrorCode,
}

impl Sendable for ErrorMsg {
    fn serialize(
        self,
        data: DataWriter<'_>,
        _handles: HandlesWriter<'_>,
    ) -> Result<MessageInfo, ErrorCode> {
        let len = data.header_only(RawError {
            reason: self.reason,
        });
        Ok(MessageInfo::new(MessageKind::Error as i32, len as u16, 0))
    }
}

impl<'a> Receivable<'a> for ErrorMsg {
    fn deserialize(
        msginfo: MessageInfo,
        data: DataReader<'a>,
        _handles: HandlesReader<'a>,
    ) -> Option<Self> {
        let raw: &RawError = data.header_only(msginfo);
        Some(ErrorMsg { reason: raw.reason })
    }
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

pub enum Message2<'a> {
    Connect(ConnectMsg),
    Open((CallId, OpenMsg<'a>)),
    OpenReply((CallId, OpenReplyMsg)),
    FramedData(FramedDataMsg<'a>),
    StreamData(StreamDataMsg<'a>),
    Abort((CallId, AbortMsg)),
    Error(ErrorMsg),
}

pub struct AnyMessage {
    pub msginfo: MessageInfo,
    pub buffer: OwnedMessageBuffer,
}

impl AnyMessage {
    pub unsafe fn new(buffer: OwnedMessageBuffer, msginfo: MessageInfo) -> Self {
        Self { buffer, msginfo }
    }

    pub fn parse(&mut self) -> Option<Message2<'_>> {
        match self.msginfo.kind() {
            // FIXME: Verify the # of handles too.
            kind if kind == MessageKind::Connect as usize => {
                Receivable::deserialize_from_buffer(self.msginfo, &mut self.buffer)
                    .map(Message2::Connect)
            }
            kind if kind == MessageKind::Open as usize => {
                Receivable::deserialize_from_buffer(self.msginfo, &mut self.buffer)
                    .map(Message2::Open)
            }
            kind if kind == MessageKind::OpenReply as usize => {
                Receivable::deserialize_from_buffer(self.msginfo, &mut self.buffer)
                    .map(Message2::OpenReply)
            }
            kind if kind == MessageKind::FramedData as usize => {
                Receivable::deserialize_from_buffer(self.msginfo, &mut self.buffer)
                    .map(Message2::FramedData)
            }
            kind if kind == MessageKind::StreamData as usize => {
                Receivable::deserialize_from_buffer(self.msginfo, &mut self.buffer)
                    .map(Message2::StreamData)
            }
            kind if kind == MessageKind::Abort as usize => {
                Receivable::deserialize_from_buffer(self.msginfo, &mut self.buffer)
                    .map(Message2::Abort)
            }
            kind if kind == MessageKind::Error as usize => {
                Receivable::deserialize_from_buffer(self.msginfo, &mut self.buffer)
                    .map(Message2::Error)
            }
            _ => None,
        }
    }
}
