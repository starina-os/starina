#[cfg(feature = "userspace")]
pub mod userspace {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use core::mem::size_of;
    use core::ops::Deref;
    use core::ops::DerefMut;
    use core::ptr::NonNull;

    use crate::error::ErrorCode;
    use crate::handle::HandleId;
    use crate::handle::Handleable;
    use crate::handle::OwnedHandle;
    use crate::message::MESSAGE_DATA_LEN_MAX;
    use crate::message::MESSAGE_NUM_HANDLES_MAX;
    use crate::message::MessageInfo;
    use crate::syscall;

    pub struct Channel(OwnedHandle);

    impl Channel {
        pub fn from_handle(handle: OwnedHandle) -> Self {
            Self(handle)
        }

        pub fn send(&self, writer: impl MessageWriter) -> Result<(), ErrorCode> {
            let mut buffer = OwnedMessageBuffer::alloc();
            let msginfo = writer.write(&mut buffer)?;

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
    }

    impl Handleable for Channel {
        fn handle_id(&self) -> HandleId {
            self.0.id()
        }
    }

    pub struct MessageBuffer {
        pub data: [u8; MESSAGE_DATA_LEN_MAX],
        pub handles: [HandleId; MESSAGE_NUM_HANDLES_MAX],
    }

    impl MessageBuffer {
        pub const unsafe fn data_as_ref<T>(&self) -> &T {
            debug_assert!(size_of::<T>() <= MESSAGE_DATA_LEN_MAX);
            unsafe { &*(self.data.as_ptr() as *const T) }
        }

        pub const unsafe fn data_as_mut<T>(&mut self) -> &mut T {
            debug_assert!(size_of::<T>() <= MESSAGE_DATA_LEN_MAX);
            unsafe { &mut *(self.data.as_mut_ptr() as *mut T) }
        }
    }

    // TODO: Make this thread local.
    static GLOBAL_BUFFER_POOL: spin::Mutex<Vec<NonNull<MessageBuffer>>> =
        spin::Mutex::new(Vec::new());
    const BUFFER_POOL_SIZE_MAX: usize = 16;

    pub struct OwnedMessageBuffer(Box<MessageBuffer>);
    impl OwnedMessageBuffer {
        pub fn alloc() -> Self {
            let buffer = GLOBAL_BUFFER_POOL.lock().pop().unwrap_or_else(|| {
                // TODO: Use `MaybeUninit` to unnecesarily zero-fill the buffer.
                Box::leak(Box::new(MessageBuffer {
                    handles: [HandleId::from_raw(0); MESSAGE_NUM_HANDLES_MAX],
                    data: [0; MESSAGE_DATA_LEN_MAX],
                }))
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

            let mut pool = GLOBAL_BUFFER_POOL.lock();
            if pool.len() < BUFFER_POOL_SIZE_MAX {
                pool.push(self.0);
            }
        }
    }
    pub struct AnyMessage {
        msginfo: MessageInfo,
        buffer: OwnedMessageBuffer,
    }

    impl AnyMessage {
        unsafe fn new(buffer: OwnedMessageBuffer, msginfo: MessageInfo) -> Self {
            Self { buffer, msginfo }
        }
    }

    pub trait MessageWriter {
        fn write(&self, buffer: &mut MessageBuffer) -> Result<MessageInfo, ErrorCode>;
    }

    pub mod message {
        use super::AnyMessage;
        use super::MessageBuffer;
        use super::MessageWriter;
        use super::OwnedMessageBuffer;
        use crate::error::ErrorCode;
        use crate::message::MessageInfo;

        #[repr(C)]
        struct RawPing {
            value: u32,
        }

        pub struct PingWriter {
            pub value: u32,
        }

        impl MessageWriter for PingWriter {
            fn write(&self, buffer: &mut MessageBuffer) -> Result<MessageInfo, ErrorCode> {
                let raw = RawPing { value: self.value };
                unsafe {
                    core::ptr::write(buffer.data_as_mut::<RawPing>(), raw);
                }

                Ok(MessageInfo::new(
                    1,
                    size_of::<RawPing>().try_into().unwrap(),
                    0,
                ))
            }
        }

        pub struct PingReader(OwnedMessageBuffer);

        impl PingReader {
            pub fn new(buffer: OwnedMessageBuffer) -> Self {
                Self(buffer)
            }

            pub fn value(&self) -> u32 {
                unsafe { self.0.data_as_ref::<RawPing>().value }
            }
        }

        impl TryFrom<AnyMessage> for PingReader {
            type Error = ErrorCode;

            fn try_from(msg: AnyMessage) -> Result<Self, Self::Error> {
                if msg.msginfo.kind() != 1 {
                    return Err(ErrorCode::InvalidMessageKind);
                }

                let buffer = msg.buffer;
                Ok(Self(buffer))
            }
        }
    }
}
