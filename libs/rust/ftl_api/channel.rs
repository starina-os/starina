//! A message-passing channel.
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::fmt;
use core::mem;

use ftl_inlinedvec::InlinedVec;
use ftl_types::error::FtlError;
use ftl_types::idl::HandleField;
use ftl_types::idl::MovedHandle;
use ftl_types::message::MessageBuffer;
use ftl_types::message::MessageCallable;
use ftl_types::message::MessageDeserialize;
use ftl_types::message::MessageInfo;
use ftl_types::message::MessageSerialize;
use ftl_types::message::MESSAGE_HANDLES_MAX_COUNT;

use crate::handle::OwnedHandle;
use crate::syscall;

/// Error type for receive operations.
#[derive(Debug, PartialEq, Eq)]
pub enum RecvError {
    Syscall(FtlError),
    Unexpected(MessageInfo),
}

/// Error type for send-then-receive operations.
#[derive(Debug, PartialEq, Eq)]
pub enum CallError {
    Syscall(FtlError),
    Unexpected(MessageInfo),
}

/// An asynchronous, bounded, and bi-directional message-passing mechanism between
/// processes.
pub struct Channel {
    handle: OwnedHandle,
}

fn process_received_message<M: MessageDeserialize>(
    msgbuffer: &mut MessageBuffer,
    msginfo: MessageInfo,
) -> Result<M::Reader<'_>, MessageInfo> {
    // FIXME: Due to a possibly false-positive borrow check issue, we can't
    //        use `buffer` anymore in the Option::ok_or_else below. This is a
    //        naive workaround.
    let mut handles: InlinedVec<_, MESSAGE_HANDLES_MAX_COUNT> = InlinedVec::new();
    for i in 0..msginfo.num_handles() {
        let handle_id = msgbuffer.handle_id(i);
        handles.try_push(handle_id).unwrap();
    }

    M::deserialize(msgbuffer, msginfo).ok_or_else(|| {
        // Close transferred handles to prevent resource leaks.
        //
        // Also, if they're IPC-related handles like channels, this might
        // let the sender know that we don't never use them. Otherwise, the
        // sender might be waiting for a message from us.
        for handle_id in handles {
            syscall::handle_close(handle_id).expect("failed to close handle");
        }

        msginfo
    })
}

fn use_temporary_msgbuffer<F, R>(f: F) -> R
where
    F: FnOnce(&mut MessageBuffer) -> R,
{
    // FIXME: Use a thread-local storage not to block other threads.
    static CACHED_BUFFER: spin::Mutex<Option<Box<MessageBuffer>>> = spin::Mutex::new(None);

    // Try to reuse the buffer to avoid memory allocation.
    let mut msgbuffer = CACHED_BUFFER
        .lock()
        .take()
        .unwrap_or_else(|| Box::new(MessageBuffer::new()));

    let ret = f(&mut msgbuffer);

    // Save the allocated buffer for later reuse.
    CACHED_BUFFER.lock().replace(msgbuffer);

    ret
}

impl Channel {
    /// Creates a new channel from a handle.
    pub fn from_handle(handle: OwnedHandle) -> Channel {
        Channel { handle }
    }

    /// Creates a new channel pair, connected to each other.
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

    /// Returns the handle of the channel.
    pub fn handle(&self) -> &OwnedHandle {
        &self.handle
    }

    /// Splits the channel into sender/receiver halves.
    ///
    /// Currently, it's no more than `Arc<Channel>`, but splitting a channel
    /// whenever you can is recommended for future compatibility.
    pub fn split(self) -> (ChannelSender, ChannelReceiver) {
        let ch = Arc::new(self);
        let sender = ChannelSender { ch: ch.clone() };
        let receiver = ChannelReceiver { ch };
        (sender, receiver)
    }

    /// Sends a message to the channel's peer. Non-blocking.
    ///
    /// # Note
    ///
    /// If the peer's message queue is full, this method will return an error
    /// immediately without blocking.
    pub fn send<M: MessageSerialize>(&self, msg: M) -> Result<(), FtlError> {
        use_temporary_msgbuffer(move |msgbuffer| self.send_with_buffer(msgbuffer, msg))
    }

    /// Sends a message to the channel's peer using the provided buffer. Non-blocking.
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

    /// Receives a message from the channel's peer. Non-blocking.
    ///
    /// See [`Self::recv`] for more details.
    pub fn try_recv<'a, M: MessageDeserialize>(
        &self,
        buffer: &'a mut MessageBuffer,
    ) -> Result<Option<M::Reader<'a>>, RecvError> {
        // TODO: Optimize parameter order to avoid unnecessary register swaps.
        let msginfo = match syscall::channel_try_recv(self.handle.id(), buffer) {
            Ok(msginfo) => msginfo,
            Err(FtlError::WouldBlock) => return Ok(None),
            Err(err) => return Err(RecvError::Syscall(err)),
        };

        let msg = process_received_message::<M>(buffer, msginfo).map_err(RecvError::Unexpected)?;
        Ok(Some(msg))
    }

    /// Receives a message from the channel's peer using the provided buffer. Blocking.
    ///
    /// Kernel writes the received message into the buffer (`msgbuffer`), this library
    /// deserializes the message, and returns a typed message object.
    ///
    /// # Example
    ///
    /// ```
    /// use ftl_api::types::message::MessageBuffer;
    ///
    /// let mut msgbuffer = MessageBuffer::new();
    /// let reply = ch.recv::<PingReply>(&mut msgbuffer);
    /// debug!("reply = {}", reply.value);
    /// ```
    pub fn recv<'a, M: MessageDeserialize>(
        &self,
        msgbuffer: &'a mut MessageBuffer,
    ) -> Result<M::Reader<'a>, RecvError> {
        // TODO: Optimize parameter order to avoid unnecessary register swaps.
        let msginfo =
            syscall::channel_recv(self.handle.id(), msgbuffer).map_err(RecvError::Syscall)?;

        let msg =
            process_received_message::<M>(msgbuffer, msginfo).map_err(RecvError::Unexpected)?;
        Ok(msg)
    }

    /// Send a message and then receive a reply. Blocking.
    ///
    /// See [`Self::recv`] for more details on `buffer`.
    pub fn call<'a, M>(
        &self,
        request: M,
        msgbuffer: &'a mut MessageBuffer,
    ) -> Result<<M::Reply as MessageDeserialize>::Reader<'a>, CallError>
    where
        M: MessageCallable,
    {
        request.serialize(msgbuffer);
        let msginfo = syscall::channel_call(self.handle.id(), M::MSGINFO, msgbuffer)
            .map_err(CallError::Syscall)?;
        let reply = process_received_message::<M::Reply>(msgbuffer, msginfo)
            .map_err(CallError::Unexpected)?;
        Ok(reply)
    }
}

impl From<Channel> for (ChannelSender, ChannelReceiver) {
    fn from(channel: Channel) -> (ChannelSender, ChannelReceiver) {
        channel.split()
    }
}

impl fmt::Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Channel({:?})", self.handle)
    }
}

impl From<MovedHandle> for Channel {
    fn from(moved_handle: MovedHandle) -> Channel {
        let handle_id = moved_handle.handle_id();
        Channel {
            handle: OwnedHandle::from_raw(handle_id),
        }
    }
}

impl From<Channel> for MovedHandle {
    fn from(channel: Channel) -> MovedHandle {
        let handle_id = channel.handle.id();
        mem::forget(channel);
        MovedHandle::new(handle_id)
    }
}

impl From<Channel> for HandleField {
    fn from(channel: Channel) -> HandleField {
        HandleField::from(MovedHandle::from(channel))
    }
}

/// The sender half of a channel. Only send operations are allowed.

#[derive(Debug)]
pub struct ChannelReceiver {
    ch: Arc<Channel>,
}

impl ChannelReceiver {
    pub fn handle(&self) -> &OwnedHandle {
        self.ch.handle()
    }

    pub fn recv<'a, M: MessageDeserialize>(
        &self,
        buffer: &'a mut MessageBuffer,
    ) -> Result<M::Reader<'a>, RecvError> {
        self.ch.recv::<M>(buffer)
    }

    pub fn try_recv<'a, M: MessageDeserialize>(
        &self,
        buffer: &'a mut MessageBuffer,
    ) -> Result<Option<M::Reader<'a>>, RecvError> {
        self.ch.try_recv::<M>(buffer)
    }
}

/// The receiver half of a channel. Only receive operations are allowed.
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
