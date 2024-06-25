use ftl_types::error::FtlError;
use ftl_types::message::MessageBody;
use ftl_types::message::MessageBuffer;
use ftl_types::message::MessageInfo;

use crate::handle::OwnedHandle;
use crate::syscall;

#[derive(Debug, PartialEq, Eq)]
pub enum RecvError {
    KernelError(FtlError),
    UnexpectedMessageType(MessageInfo),
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

    pub fn send_with_buffer<M: MessageBody>(
        &self,
        buffer: &mut MessageBuffer,
        msg: M,
    ) -> Result<(), FtlError> {
        unsafe {
            buffer.write(msg);
        }

        // TODO: return send error to keep owning handles
        // TODO: Optimize parameter order to avoid unnecessary register swaps.
        syscall::channel_send(self.handle.id(), M::MSGINFO, buffer)
    }

    pub fn recv_with_buffer<'a, M: MessageBody>(
        &self,
        buffer: &'a mut MessageBuffer,
    ) -> Result<M::Reader<'a>, RecvError> {
        // TODO: Optimize parameter order to avoid unnecessary register swaps.
        let msginfo =
            syscall::channel_recv(self.handle.id(), buffer).map_err(RecvError::KernelError)?;

        // Is it really the message we're expecting?
        if msginfo != M::MSGINFO {
            // Close transferred handles to prevent resource leaks.
            //
            // Also, if they're IPC-related handles like channels, this might
            // let the sender know that we don't never use them. Otherwise, the
            // sender might be waiting for a message from us.
            for i in 0..msginfo.num_handles() {
                let handle_id = buffer.handles[i];
                syscall::handle_close(handle_id).expect("failed to close handle");
            }

            return Err(RecvError::UnexpectedMessageType(msginfo));
        }

        Ok(M::deserialize(buffer))
    }
}
