use alloc::boxed::Box;
use alloc::sync::Arc;
use core::fmt;
use core::mem;

use ftl_types::error::FtlError;
use ftl_types::message::MessageBuffer;
use ftl_types::message::MessageDeserialize;
use ftl_types::message::MessageInfo;
use ftl_types::message::MessageSerialize;
use ftl_types::message::MovedHandle;

use crate::handle::OwnedHandle;
use crate::syscall;

#[derive(Debug, PartialEq, Eq)]
pub enum RecvError {
    Syscall(FtlError),
    Deserialize(MessageInfo),
}

pub struct Channel {
    handle: OwnedHandle,
}

impl Channel {
    pub fn from_handle(handle: OwnedHandle) -> Channel {
        Channel { handle }
    }

    pub fn create() -> Result<(Channel, Channel), FtlError> {
        let (handle0, handle1) = syscall::channel_create()?;
        let ch0 = Channel {
            handle: OwnedHandle::from_raw(handle0),
        };
        let ch1 = Channel {
            handle: OwnedHandle::from_raw(handle1),
        };

        Ok((ch0, ch1))
    }

    pub fn handle(&self) -> &OwnedHandle {
        &self.handle
    }

    pub fn split(self) -> (ChannelSender, ChannelReceiver) {
        let ch = Arc::new(self);
        let sender = ChannelSender { ch: ch.clone() };
        let receiver = ChannelReceiver { ch };
        (sender, receiver)
    }

    pub fn send<M: MessageSerialize>(&self, msg: M) -> Result<(), FtlError> {
        static CACHED_BUFFER: spin::Mutex<Option<Box<MessageBuffer>>> = spin::Mutex::new(None);

        // Try to reuse the buffer to avoid memory allocation.
        let mut msgbuffer = CACHED_BUFFER
            .lock()
            .take()
            .unwrap_or_else(|| Box::new(MessageBuffer::new()));

        let ret = self.send_with_buffer(&mut *msgbuffer, msg);

        // Save the allocated buffer for later reuse.
        CACHED_BUFFER.lock().replace(msgbuffer);
        ret
    }

    pub fn send_with_buffer<M: MessageSerialize>(
        &self,
        buffer: &mut MessageBuffer,
        msg: M,
    ) -> Result<(), FtlError> {
        msg.serialize(buffer);

        // TODO: return send error to keep owning handles
        // TODO: Optimize parameter order to avoid unnecessary register swaps.
        syscall::channel_send(self.handle.id(), M::MSGINFO, buffer)
    }

    pub fn recv_with_buffer<'a, M: MessageDeserialize>(
        &self,
        buffer: &'a mut MessageBuffer,
    ) -> Result<M::Reader<'a>, RecvError> {
        // TODO: Optimize parameter order to avoid unnecessary register swaps.
        let msginfo =
            syscall::channel_recv(self.handle.id(), buffer).map_err(RecvError::Syscall)?;

        match M::deserialize(buffer, msginfo) {
            Some(msg) => {
                return Ok(msg);
            }
            None => {
                // Close transferred handles to prevent resource leaks.
                //
                // Also, if they're IPC-related handles like channels, this might
                // let the sender know that we don't never use them. Otherwise, the
                // sender might be waiting for a message from us.
                for i in 0..msginfo.num_handles() {
                    let handle_id = buffer.handles[i];
                    syscall::handle_close(handle_id).expect("failed to close handle");
                }

                return Err(RecvError::Deserialize(msginfo));
            }
        }
    }
}

impl fmt::Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Channel({:?})", self.handle)
    }
}

impl From<Channel> for MovedHandle {
    fn from(channel: Channel) -> MovedHandle {
        let handle_id = channel.handle.id();
        mem::forget(channel);
        MovedHandle(handle_id)
    }
}

#[derive(Debug)]
pub struct ChannelReceiver {
    ch: Arc<Channel>,
}

impl ChannelReceiver {
    pub fn handle(&self) -> &OwnedHandle {
        self.ch.handle()
    }

    pub fn recv_with_buffer<'a, M: MessageDeserialize>(
        &self,
        buffer: &'a mut MessageBuffer,
    ) -> Result<M::Reader<'a>, RecvError> {
        self.ch.recv_with_buffer::<M>(buffer)
    }
}

#[derive(Debug, Clone)]
pub struct ChannelSender {
    ch: Arc<Channel>,
}

impl ChannelSender {
    pub fn handle(&self) -> &OwnedHandle {
        self.ch.handle()
    }

    pub fn send<M: MessageSerialize>(&self, msg: M) -> Result<(), FtlError> {
        self.ch.send(msg)
    }

    pub fn send_with_buffer<M: MessageSerialize>(
        &self,
        buffer: &mut MessageBuffer,
        msg: M,
    ) -> Result<(), FtlError> {
        self.ch.send_with_buffer(buffer, msg)
    }
}
