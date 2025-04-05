use alloc::boxed::Box;
use core::ops::Deref;
use core::ops::DerefMut;

use starina_types::handle::HandleId;
pub use starina_types::message::*;

use crate::syscall;

pub struct OwnedMessageBuffer(Box<MessageBuffer>);

impl OwnedMessageBuffer {
    pub fn alloc() -> Self {
        // TODO: Have a thread-local buffer pool.
        // TODO: Use `MaybeUninit` to unnecesarily zero-fill the buffer.
        let buffer = Box::new(MessageBuffer::zeroed());
        OwnedMessageBuffer(buffer)
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
        // FIXME: Drop handles.
        // for handle in self.0.handles() {
        //     if handle.as_raw() != 0 {
        //         if let Err(e) = syscall::handle_close(*handle) {
        //             debug_warn!("failed to close handle: {:?}", e);
        //         }
        //     }
        // }
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
