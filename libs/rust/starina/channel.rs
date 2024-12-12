use core::fmt;

use alloc::sync::Arc;
use starina_types::error::ErrorCode;
use starina_types::message::{Deserialize, MessageBuffer};

use crate::handle::OwnedHandle;
use crate::syscall;

#[derive(Debug, PartialEq, Eq)]
pub enum RecvError {
    Syscall(ErrorCode),
    Deserialize(ErrorCode),
}

/// An asynchronous, bounded, and bi-directional message-passing mechanism between
/// processes.
pub struct Channel {
    handle: OwnedHandle,
}

impl Channel {
    /// Creates a new channel from a handle.
    pub fn from_handle(handle: OwnedHandle) -> Channel {
        Channel { handle }
    }

    /// Receives a message from the channel's peer. Non-blocking.
    ///
    /// See [`Self::recv`] for more details.
    pub fn try_recv<'a, M: Deserialize>(
        &self,
        buffer: &'a mut MessageBuffer,
    ) -> Result<Option<M>, RecvError> {
        // TODO: Optimize parameter order to avoid unnecessary register swaps.
        let msginfo = match syscall::channel_try_recv(self.handle.id(), buffer) {
            Ok(msginfo) => msginfo,
            Err(ErrorCode::WouldBlock) => return Ok(None),
            Err(err) => return Err(RecvError::Syscall(err)),
        };

        let msg = M::deserialize(buffer).map_err(RecvError::Deserialize)?;
        Ok(Some(msg))
    }
}

impl fmt::Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Channel({:?})", self.handle)
    }
}

#[derive(Debug)]
pub struct ChannelSender {}

#[derive(Debug)]
pub struct ChannelReceiver {
    ch: Arc<Channel>,
}

impl ChannelReceiver {
    pub fn try_recv<'a, M: Deserialize>(
        &self,
        buffer: &'a mut MessageBuffer,
    ) -> Result<Option<M>, RecvError> {
        self.ch.try_recv::<M>(buffer)
    }
}
