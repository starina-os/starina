use alloc::boxed::Box;
use core::marker::PhantomData;
use core::ops::Deref;
use core::ops::DerefMut;

use starina_types::handle::HandleId;
pub use starina_types::message::*;

use crate::handle::OwnedHandle;
use crate::syscall;

pub struct OwnedMessageBuffer(Box<MessageBuffer>);
impl OwnedMessageBuffer {
    pub fn alloc() -> Self {
        // TODO: Have a thread-local buffer pool.
        // TODO: Use `MaybeUninit` to unnecesarily zero-fill the buffer.
        let buffer = Box::new(MessageBuffer {
            handles: [HandleId::from_raw(0); MESSAGE_NUM_HANDLES_MAX],
            data: [0; MESSAGE_DATA_LEN_MAX],
        });

        OwnedMessageBuffer(buffer)
    }

    pub fn take_handle(&mut self, index: usize) -> Option<HandleId> {
        let handle = self.0.handles[index];
        if handle.as_raw() == 0 {
            return None;
        }

        self.0.handles[index] = HandleId::from_raw(0);
        Some(handle)
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
        // Drop handles.
        for handle in self.0.handles.iter() {
            if handle.as_raw() != 0 {
                if let Err(e) = syscall::handle_close(*handle) {
                    warn!("failed to close handle: {:?}", e);
                }
            }
        }
    }
}

pub struct Message<M: Messageable> {
    msginfo: MessageInfo,
    buffer: OwnedMessageBuffer,
    _pd: PhantomData<M>,
}

impl<M: Messageable> TryFrom<AnyMessage> for Message<M> {
    type Error = AnyMessage;

    fn try_from(msg: AnyMessage) -> Result<Self, AnyMessage> {
        if !unsafe { M::is_valid(msg.msginfo, &msg.buffer) } {
            return Err(msg);
        }

        Ok(Message {
            msginfo: msg.msginfo,
            buffer: msg.buffer,
            _pd: PhantomData,
        })
    }
}

impl Message<Connect> {
    pub fn handle(&mut self) -> Option<OwnedHandle> {
        let id = self.buffer.take_handle(0)?;
        Some(OwnedHandle::from_raw(id))
    }
}

impl<'a> Message<Open<'a>> {
    pub fn uri(&self) -> &str {
        // SAFETY: The validity of the message is checked in `Message::new`.
        unsafe { Open::cast_unchecked(self.msginfo, &self.buffer).uri }
    }
}

impl<'a> Message<FramedData<'a>> {
    pub fn data(&self) -> &[u8] {
        // SAFETY: The validity of the message is checked in `Message::new`.
        unsafe { FramedData::cast_unchecked(self.msginfo, &self.buffer).data }
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
