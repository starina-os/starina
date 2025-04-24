use alloc::sync::Arc;

use starina_types::message::MessageInfo;

use crate::error::ErrorCode;
use crate::handle::HandleId;
use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::message::AnyMessage;
use crate::message::CallId;
use crate::message::Callable;
use crate::message::OwnedMessageBuffer;
use crate::message::Replyable;
use crate::message::Sendable;
use crate::syscall;

#[derive(Debug)]
pub struct Channel(OwnedHandle);

impl Channel {
    pub fn new() -> Result<(Self, Self), ErrorCode> {
        let (first, second) = syscall::channel_create()?;
        let first = Channel(OwnedHandle::from_raw(first));
        let second = Channel(OwnedHandle::from_raw(second));
        Ok((first, second))
    }

    pub fn from_handle(handle: OwnedHandle) -> Self {
        Self(handle)
    }

    pub fn send(&self, msg: impl Sendable) -> Result<(), ErrorCode> {
        let mut buffer = OwnedMessageBuffer::alloc();
        let msginfo = msg.serialize_to_buffer(&mut buffer)?;
        self.do_send(msginfo, buffer)
    }

    pub fn call(&self, call_id: CallId, msg: impl Callable) -> Result<(), ErrorCode> {
        let mut buffer = OwnedMessageBuffer::alloc();
        let msginfo = msg.serialize_to_buffer(call_id, &mut buffer)?;
        self.do_send(msginfo, buffer)
    }

    pub fn reply(&self, call_id: CallId, msg: impl Replyable) -> Result<(), ErrorCode> {
        let mut buffer = OwnedMessageBuffer::alloc();
        let msginfo = msg.serialize_to_buffer(call_id, &mut buffer)?;
        self.do_send(msginfo, buffer)
    }

    fn do_send(&self, msginfo: MessageInfo, buffer: OwnedMessageBuffer) -> Result<(), ErrorCode> {
        syscall::channel_send(
            self.0.id(),
            msginfo,
            buffer.data_ptr(),
            buffer.handles_ptr(),
        )?;

        buffer.forget_handles();

        Ok(())
    }

    pub fn recv(&self) -> Result<AnyMessage, ErrorCode> {
        let mut buffer = OwnedMessageBuffer::alloc();
        let msginfo =
            syscall::channel_recv(self.0.id(), buffer.data_mut_ptr(), buffer.handles_mut_ptr())?;

        let msg = unsafe { AnyMessage::new(buffer, msginfo) };
        Ok(msg)
    }

    pub fn split(self) -> (ChannelSender, ChannelReceiver) {
        let ch = Arc::new(self);
        let sender = ChannelSender(ch.clone());
        let receiver = ChannelReceiver(ch);
        (sender, receiver)
    }
}

impl Handleable for Channel {
    fn handle_id(&self) -> HandleId {
        self.0.id()
    }
}

impl<'de> serde::Deserialize<'de> for Channel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let handle_id: i32 = serde::Deserialize::deserialize(deserializer)?;
        let handle = OwnedHandle::from_raw(HandleId::from_raw(handle_id));
        Ok(Channel(handle))
    }
}

#[derive(Debug, Clone)]
pub struct ChannelSender(Arc<Channel>);

#[derive(Debug)]
pub struct ChannelReceiver(Arc<Channel>);

impl ChannelSender {
    pub fn send(&self, msg: impl Sendable) -> Result<(), ErrorCode> {
        self.0.send(msg)
    }

    pub fn reply(&self, call_id: CallId, msg: impl Replyable) -> Result<(), ErrorCode> {
        self.0.reply(call_id, msg)
    }

    pub fn handle(&self) -> &OwnedHandle {
        &self.0.0
    }
}

impl ChannelReceiver {
    pub fn recv(&self) -> Result<AnyMessage, ErrorCode> {
        self.0.recv()
    }

    pub fn handle(&self) -> &OwnedHandle {
        &self.0.0
    }
}
