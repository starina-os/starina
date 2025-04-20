use alloc::boxed::Box;
use core::mem::MaybeUninit;
use core::ops::Deref;
use core::ops::DerefMut;

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
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct CallId(pub u32);

impl From<u32> for CallId {
    fn from(value: u32) -> Self {
        CallId(value)
    }
}

fn parse_str(s: &[u8]) -> Option<&str> {
    // Check if the slice is valid ASCII string.
    if s.iter().all(|&b| b.is_ascii()) {
        // SAFETY: The slice is valid UTF-8.
        Some(unsafe { core::str::from_utf8_unchecked(s) })
    } else {
        None
    }
}

pub struct MessageBuffer {
    data: [MaybeUninit<u8>; MESSAGE_DATA_LEN_MAX],
    handles: [MaybeUninit<HandleId>; MESSAGE_NUM_HANDLES_MAX],
}

impl MessageBuffer {
    const fn new() -> Self {
        Self {
            data: [const { MaybeUninit::uninit() }; MESSAGE_DATA_LEN_MAX],
            handles: [const { MaybeUninit::uninit() }; MESSAGE_NUM_HANDLES_MAX],
        }
    }
}

pub trait Sendable {
    fn serialize(
        self,
        data: &mut [MaybeUninit<u8>; MESSAGE_DATA_LEN_MAX],
        handles: &mut [MaybeUninit<HandleId>; MESSAGE_NUM_HANDLES_MAX],
    ) -> Result<MessageInfo, ErrorCode>;
}

pub trait Receivable: Sized {
    fn try_deserialize(
        msginfo: MessageInfo,
        data: &[MaybeUninit<u8>; MESSAGE_DATA_LEN_MAX],
        handles: &[MaybeUninit<HandleId>; MESSAGE_NUM_HANDLES_MAX],
    ) -> Option<Self>;
}

#[repr(C)]
struct RawOpen {
    call_id: CallId,
    uri: [u8; URI_LEN_MAX],
}

impl Sendable for (CallId, OpenMsg<'_>) {
    fn serialize(
        self,
        data: &mut [MaybeUninit<u8>; MESSAGE_DATA_LEN_MAX],
        _handles: &mut [MaybeUninit<HandleId>; MESSAGE_NUM_HANDLES_MAX],
    ) -> Result<MessageInfo, ErrorCode> {
        let (call_id, OpenMsg { uri }) = self;

        if uri.len() > URI_LEN_MAX {
            return Err(ErrorCode::TooLongUri);
        }

        let raw = unsafe { &mut *(data.as_mut_ptr() as *mut RawOpen) };
        raw.call_id = call_id;
        raw.uri[..uri.len()].copy_from_slice(uri.as_bytes());

        let data_len = size_of::<CallId>() + uri.len();
        Ok(MessageInfo::new(
            MessageKind::Open as i32,
            data_len as u16,
            0,
        ))
    }
}

impl Receivable for (CallId, OpenMsg<'_>) {
    fn try_deserialize(
        msginfo: MessageInfo,
        data: &[MaybeUninit<u8>; MESSAGE_DATA_LEN_MAX],
        _handles: &[MaybeUninit<HandleId>; MESSAGE_NUM_HANDLES_MAX],
    ) -> Option<Self> {
        let raw = unsafe { &*(data.as_ptr() as *const RawOpen) };
        let uri_len = msginfo.data_len().checked_sub(size_of::<CallId>())?;
        let uri = parse_str(&raw.uri[..uri_len])?;
        Some((raw.call_id, OpenMsg { uri }))
    }
}

#[repr(C)]
struct RawOpenReply {
    call_id: CallId,
}

impl Sendable for (CallId, OpenReplyMsg) {
    fn serialize(
        self,
        data: &mut [MaybeUninit<u8>; MESSAGE_DATA_LEN_MAX],
        handles: &mut [MaybeUninit<HandleId>; MESSAGE_NUM_HANDLES_MAX],
    ) -> Result<MessageInfo, ErrorCode> {
        let (call_id, OpenReplyMsg { handle }) = self;

        let raw = unsafe { &mut *(data.as_mut_ptr() as *mut RawOpenReply) };
        raw.call_id = call_id;

        handles[0].write(handle.handle_id());
        core::mem::forget(handle);

        let data_len = size_of::<RawOpenReply>();
        Ok(MessageInfo::new(
            MessageKind::OpenReply as i32,
            data_len as u16,
            1,
        ))
    }
}

impl Receivable for (CallId, OpenReplyMsg) {
    fn try_deserialize(
        _msginfo: MessageInfo,
        data: &[MaybeUninit<u8>; MESSAGE_DATA_LEN_MAX],
        handles: &[MaybeUninit<HandleId>; MESSAGE_NUM_HANDLES_MAX],
    ) -> Option<Self> {
        let raw = unsafe { &*(data.as_ptr() as *const RawOpenReply) };
        let call_id = raw.call_id;
        let handle_id = unsafe { handles[0].assume_init() };

        let handle = OwnedHandle::from_raw(handle_id);
        let channel = Channel::from_handle(handle);

        Some((call_id, OpenReplyMsg { handle: channel }))
    }
}

// ------------------------------------------------------------

pub struct MessageBufferOld {
    data: [u8; MESSAGE_DATA_LEN_MAX],
    handles: [HandleId; MESSAGE_NUM_HANDLES_MAX],
}

impl MessageBufferOld {
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

pub trait MessageCommon {
    fn kind() -> MessageKind;
}

pub trait OneWayMessage<'a>: MessageCommon {
    fn write(self, buffer: &mut MessageBufferOld) -> Result<MessageInfo, ErrorCode>;
    unsafe fn parse_unchecked(
        msginfo: MessageInfo,
        buffer: &'a mut MessageBufferOld,
    ) -> Option<Self>
    where
        Self: Sized;
}

pub trait RequestReplyMessage<'a>: MessageCommon {
    fn write(
        self,
        call_id: CallId,
        buffer: &mut MessageBufferOld,
    ) -> Result<MessageInfo, ErrorCode>;
    unsafe fn parse_unchecked(
        msginfo: MessageInfo,
        buffer: &'a mut MessageBufferOld,
    ) -> Option<(Self, CallId)>
    where
        Self: Sized;
}

pub const URI_LEN_MAX: usize = 1024;

pub struct ConnectMsg {
    pub handle: Channel,
}

impl<'a> MessageCommon for ConnectMsg {
    fn kind() -> MessageKind {
        MessageKind::Connect
    }
}

impl<'a> OneWayMessage<'a> for ConnectMsg {
    fn write(self, buffer: &mut MessageBufferOld) -> Result<MessageInfo, ErrorCode> {
        let handle = self.handle;
        buffer.handles[0] = handle.handle_id();

        // Avoid dropping the handle. It will be moved to the channel.
        core::mem::forget(handle);

        Ok(MessageInfo::new(MessageKind::Connect as i32, 0, 1))
    }

    unsafe fn parse_unchecked(
        _msginfo: MessageInfo,
        buffer: &'a mut MessageBufferOld,
    ) -> Option<Self> {
        let handle = buffer.handles[0];
        buffer.handles[0] = HandleId::from_raw(0); // Avoid dropping the handle.

        Some(ConnectMsg {
            handle: Channel::from_handle(OwnedHandle::from_raw(handle)),
        })
    }
}

struct RawOpenOld {
    call_id: CallId,
    uri: [u8; URI_LEN_MAX],
}

pub struct OpenMsg<'a> {
    pub uri: &'a str,
}

impl<'a> RequestReplyMessage<'a> for OpenMsg<'a> {
    fn write(
        self,
        call_id: CallId,
        buffer: &mut MessageBufferOld,
    ) -> Result<MessageInfo, ErrorCode> {
        let uri_len = self.uri.len();
        let raw = unsafe { buffer.data_as_mut::<RawOpen>() };
        if uri_len > URI_LEN_MAX {
            return Err(ErrorCode::TooLongUri);
        }

        raw.call_id = call_id;
        raw.uri[..uri_len].copy_from_slice(self.uri.as_bytes());
        Ok(MessageInfo::new(
            MessageKind::Open as i32,
            uri_len as u16,
            0,
        ))
    }

    unsafe fn parse_unchecked(
        msginfo: MessageInfo,
        buffer: &'a mut MessageBufferOld,
    ) -> Option<(Self, CallId)> {
        let raw = unsafe { buffer.data_as_ref::<RawOpen>() };
        let uri = unsafe { core::str::from_utf8_unchecked(&raw.uri[..msginfo.data_len()]) };
        Some((OpenMsg { uri }, raw.call_id))
    }
}

impl<'a> MessageCommon for OpenMsg<'a> {
    fn kind() -> MessageKind {
        MessageKind::Open
    }
}

struct RawOpenReplyOld {
    call_id: CallId,
}

pub struct OpenReplyMsg {
    pub handle: Channel,
}

impl<'a> RequestReplyMessage<'a> for OpenReplyMsg {
    fn write(
        self,
        call_id: CallId,
        buffer: &mut MessageBufferOld,
    ) -> Result<MessageInfo, ErrorCode> {
        let handle = self.handle;
        buffer.handles[0] = handle.handle_id();

        let raw = unsafe { buffer.data_as_mut::<RawOpenReplyOld>() };
        raw.call_id = call_id;

        // Avoid dropping the handle. It will be moved to the channel.
        core::mem::forget(handle);

        Ok(MessageInfo::new(
            MessageKind::OpenReply as i32,
            size_of::<RawOpenReplyOld>() as u16,
            1,
        ))
    }
    unsafe fn parse_unchecked(
        _msginfo: MessageInfo,
        buffer: &'a mut MessageBufferOld,
    ) -> Option<(Self, CallId)> {
        let handle = buffer.handles[0];
        buffer.handles[0] = HandleId::from_raw(0); // Avoid dropping the handle.

        Some((
            OpenReplyMsg {
                handle: Channel::from_handle(OwnedHandle::from_raw(handle)),
            },
            unsafe { buffer.data_as_ref::<RawOpenReplyOld>() }.call_id,
        ))
    }
}

impl<'a> MessageCommon for OpenReplyMsg {
    fn kind() -> MessageKind {
        MessageKind::OpenReply
    }
}

pub struct FramedDataMsg<'a> {
    pub data: &'a [u8],
}

impl<'a> MessageCommon for FramedDataMsg<'a> {
    fn kind() -> MessageKind {
        MessageKind::FramedData
    }
}

impl<'a> OneWayMessage<'a> for FramedDataMsg<'a> {
    unsafe fn parse_unchecked(
        msginfo: MessageInfo,
        buffer: &'a mut MessageBufferOld,
    ) -> Option<Self> {
        let data = &buffer.data[..msginfo.data_len()];
        Some(FramedDataMsg { data })
    }

    fn write(self, buffer: &mut MessageBufferOld) -> Result<MessageInfo, ErrorCode> {
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

pub struct StreamDataMsg<'a> {
    pub data: &'a [u8],
}

impl<'a> MessageCommon for StreamDataMsg<'a> {
    fn kind() -> MessageKind {
        MessageKind::StreamData
    }
}

impl<'a> OneWayMessage<'a> for StreamDataMsg<'a> {
    unsafe fn parse_unchecked(
        msginfo: MessageInfo,
        buffer: &'a mut MessageBufferOld,
    ) -> Option<Self> {
        let data = &buffer.data[..msginfo.data_len()];
        Some(StreamDataMsg { data })
    }

    fn write(self, buffer: &mut MessageBufferOld) -> Result<MessageInfo, ErrorCode> {
        if self.data.len() > buffer.data.len() {
            return Err(ErrorCode::TooLarge);
        }

        buffer.data[..self.data.len()].copy_from_slice(self.data);
        Ok(MessageInfo::new(
            MessageKind::StreamData as i32,
            self.data.len() as u16,
            0,
        ))
    }
}

pub struct OwnedMessageBuffer(Box<MessageBufferOld>);

impl OwnedMessageBuffer {
    pub fn alloc() -> Self {
        // TODO: Have a thread-local buffer pool.
        // TODO: Use `MaybeUninit` to unnecesarily zero-fill the buffer.
        let buffer = Box::new(MessageBufferOld::zeroed());
        OwnedMessageBuffer(buffer)
    }

    pub fn forget_handles(mut self) {
        // TODO: Can we drop the box without calling OwnedHandle's drop?
        for handle in self.0.handles_mut() {
            *handle = HandleId::from_raw(0);
        }
    }
}

impl Deref for OwnedMessageBuffer {
    type Target = MessageBufferOld;

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
        for handle in self.0.handles() {
            if handle.as_raw() != 0 {
                if let Err(e) = syscall::handle_close(*handle) {
                    debug_warn!("failed to close handle: {:?}", e);
                }
            }
        }
    }
}

pub struct AnyMessage {
    pub msginfo: MessageInfo,
    pub buffer: OwnedMessageBuffer,
}

impl AnyMessage {
    pub unsafe fn new(buffer: OwnedMessageBuffer, msginfo: MessageInfo) -> Self {
        Self { buffer, msginfo }
    }
}
