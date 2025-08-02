use alloc::sync::Arc;

use starina_types::message::MessageInfo;

use crate::error::ErrorCode;
use crate::handle::HandleId;
use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::message::Message;
use crate::message::MessageBuffer;
use crate::syscall;

#[derive(Debug)]
pub enum RecvError {
    Parse(MessageInfo),
    Syscall(ErrorCode),
}

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

    pub fn send(&self, msg: Message<'_>) -> Result<(), ErrorCode> {
        let mut msgbuffer = MessageBuffer::new();
        let msginfo = msg.serialize(&mut msgbuffer)?;
        self.do_send(msginfo, msgbuffer)
    }

    fn do_send(&self, msginfo: MessageInfo, msg: MessageBuffer) -> Result<(), ErrorCode> {
        syscall::channel_send(self.0.id(), msginfo, msg.data_ptr(), msg.handles_ptr())?;
        Ok(())
    }

    pub fn recv<'a>(&self, buffer: &'a mut MessageBuffer) -> Result<Message<'a>, RecvError> {
        let msginfo =
            syscall::channel_recv(self.0.id(), buffer.data_mut_ptr(), buffer.handles_mut_ptr())
                .map_err(RecvError::Syscall)?;
        Message::deserialize(msginfo, buffer).ok_or(RecvError::Parse(msginfo))
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
    pub fn send(&self, msg: Message<'_>) -> Result<(), ErrorCode> {
        self.0.send(msg)
    }
}

impl Handleable for ChannelSender {
    fn handle_id(&self) -> HandleId {
        self.0.handle_id()
    }
}

impl ChannelReceiver {
    pub fn recv<'a>(&self, buffer: &'a mut MessageBuffer) -> Result<Message<'a>, RecvError> {
        self.0.recv(buffer)
    }
}

impl Handleable for ChannelReceiver {
    fn handle_id(&self) -> HandleId {
        self.0.handle_id()
    }
}

impl Into<(ChannelSender, ChannelReceiver)> for Channel {
    fn into(self) -> (ChannelSender, ChannelReceiver) {
        self.split()
    }
}
