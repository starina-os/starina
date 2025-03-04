#[cfg(feature = "userspace")]
pub mod userspace {
    use alloc::boxed::Box;
    use alloc::vec::Vec;

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
    }

    impl Handleable for Channel {
        fn handle_id(&self) -> HandleId {
            self.0.id()
        }
    }

    struct RawMessageBuffer {
        pub buffer: [u8; MESSAGE_DATA_LEN_MAX],
        pub handles: [HandleId; MESSAGE_NUM_HANDLES_MAX],
    }

    static GLOBAL_BUFFER_POOL: spin::Mutex<Vec<Box<RawMessageBuffer>>> =
        spin::Mutex::new(Vec::new());

    pub struct MessageBuffer {
        buffer: *mut RawMessageBuffer,
    }

    impl MessageBuffer {
        pub fn alloc() -> MessageBuffer {
            let buffer = if let Some(buffer) = GLOBAL_BUFFER_POOL.lock().pop() {
                Box::into_raw(buffer)
            } else {
                // TODO: Skip zeroing out the buffer.
                Box::into_raw(Box::new(RawMessageBuffer {
                    buffer: [0; MESSAGE_DATA_LEN_MAX],
                    handles: [HandleId::from_raw(0); MESSAGE_NUM_HANDLES_MAX],
                }))
            };
            MessageBuffer { buffer }
        }
    }

    impl Drop for MessageBuffer {
        fn drop(&mut self) {
            unsafe {
                GLOBAL_BUFFER_POOL.lock().push(Box::from_raw(self.buffer));
            }
        }
    }
}
