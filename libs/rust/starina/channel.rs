#[cfg(feature = "userspace")]
pub mod userspace {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use core::mem::MaybeUninit;

    use crate::error::ErrorCode;
    use crate::handle::HandleId;
    use crate::handle::Handleable;
    use crate::handle::OwnedHandle;
    use crate::message::MESSAGE_DATA_LEN_MAX;
    use crate::message::MESSAGE_NUM_HANDLES_MAX;

    pub struct Channel(OwnedHandle);

    impl Channel {
        pub fn from_handle(handle: OwnedHandle) -> Self {
            Self(handle)
        }

        pub fn recv(&self) -> Result<AnyMessage, ErrorCode> {
            todo!()
        }
    }

    impl Handleable for Channel {
        fn handle_id(&self) -> HandleId {
            self.0.id()
        }
    }

    struct MessageBuffer {
        pub data: [u8; MESSAGE_DATA_LEN_MAX],
        pub handles: [HandleId; MESSAGE_NUM_HANDLES_MAX],
    }

    // TODO: Make this thread local.
    static GLOBAL_BUFFER_POOL: spin::Mutex<Vec<Box<MaybeUninit<MessageBuffer>>>> =
        spin::Mutex::new(Vec::new());
    const BUFFER_POOL_SIZE_MAX: usize = 16;

    pub struct AnyMessage {
        // msginfo: MessageInfo,
        buffer: *mut MaybeUninit<MessageBuffer>,
    }

    impl AnyMessage {
        pub fn alloc() -> AnyMessage {
            let buffer = if let Some(buffer) = GLOBAL_BUFFER_POOL.lock().pop() {
                Box::into_raw(buffer)
            } else {
                // TODO: Skip zeroing out the buffer.
                Box::into_raw(Box::new(MaybeUninit::uninit()))
            };
            AnyMessage { buffer }
        }

        pub unsafe fn raw_buffer(&self) -> *mut MessageBuffer {
            unsafe { (*self.buffer).as_mut_ptr() }
        }
    }

    impl Drop for AnyMessage {
        fn drop(&mut self) {
            let buffer = unsafe { Box::from_raw(self.buffer) };
            let mut pool = GLOBAL_BUFFER_POOL.lock();
            if pool.len() < BUFFER_POOL_SIZE_MAX {
                pool.push(buffer);
            }
        }
    }
}
