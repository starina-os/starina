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
    pub fn new() -> Result<(Self, Self), ErrorCode> {
        let (first, second) = syscall::channel_create()?;
        let first = Channel(OwnedHandle::from_raw(first));
        let second = Channel(OwnedHandle::from_raw(second));
        Ok((first, second))
    }

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
