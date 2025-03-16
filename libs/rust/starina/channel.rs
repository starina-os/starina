use alloc::sync::Arc;

use crate::error::ErrorCode;
use crate::handle::HandleId;
use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::message::AnyMessage;
use crate::message::Messageable;
use crate::message::OwnedMessageBuffer;
use crate::syscall;

#[derive(Debug)]
pub struct Channel(OwnedHandle);

impl Channel {
    pub fn from_handle(handle: OwnedHandle) -> Self {
        Self(handle)
    }

    pub fn send(&self, msg: impl Messageable) -> Result<(), ErrorCode> {
        let mut buffer = OwnedMessageBuffer::alloc();
        let msginfo = msg.write(&mut buffer)?;

        syscall::channel_send(
            self.0.id(),
            msginfo,
            buffer.data.as_ptr(),
            buffer.handles.as_ptr(),
        )?;
        Ok(())
    }

    pub fn recv(&self) -> Result<AnyMessage, ErrorCode> {
        let mut buffer = OwnedMessageBuffer::alloc();
        let data_ptr = buffer.data.as_mut_ptr();
        let handles_ptr = buffer.handles.as_mut_ptr();
        let msginfo = syscall::channel_recv(self.0.id(), data_ptr, handles_ptr)?;

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

impl serde::Serialize for Channel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i32(self.0.id().as_raw())
    }
}

impl<'de> serde::Deserialize<'de> for Channel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: i32 = serde::Deserialize::deserialize(deserializer)?;
        let id = HandleId::from_raw(raw);
        Ok(Channel::from_handle(OwnedHandle::from_raw(id)))
    }
}

#[derive(Debug, Clone)]
pub struct ChannelSender(Arc<Channel>);

#[derive(Debug)]
pub struct ChannelReceiver(Arc<Channel>);

impl ChannelSender {
    pub fn send(&self, writer: impl Messageable) -> Result<(), ErrorCode> {
        self.0.send(writer)
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
