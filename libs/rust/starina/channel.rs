#[cfg(feature = "userspace")]
pub mod userspace {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use core::mem::MaybeUninit;
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

        pub fn recv(&self) -> Result<AnyMessage, ErrorCode> {
            let buffer = MessageBuffer::alloc();
            let buffer_ptr: *mut MaybeUninit<MessageBuffer> = buffer.as_ptr();
            let buffer_ptr = unsafe { (*buffer_ptr).as_mut_ptr() };
            let data_ptr = unsafe { &mut (*buffer_ptr).data };
            let handles_ptr = unsafe { &mut (*buffer_ptr).handles };
            let msginfo = syscall::channel_recv(
                self.0.id(),
                data_ptr as *mut u8,
                handles_ptr as *mut HandleId,
            )?;

            let msg = unsafe { AnyMessage::new(buffer, msginfo) };
            Ok(msg)
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

    impl MessageBuffer {
        pub fn alloc() -> NonNull<MaybeUninit<MessageBuffer>> {
            if let Some(buffer) = GLOBAL_BUFFER_POOL.lock().pop() {
                unsafe { NonNull::new_unchecked(Box::into_raw(buffer)) }
            } else {
                unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(MaybeUninit::uninit()))) }
            }
        }
    }

    pub struct AnyMessage {
        msginfo: MessageInfo,
        buffer: NonNull<MaybeUninit<MessageBuffer>>,
    }

    impl AnyMessage {
        unsafe fn new(buffer: NonNull<MaybeUninit<MessageBuffer>>, msginfo: MessageInfo) -> Self {
            Self { buffer, msginfo }
        }
    }

    impl Drop for AnyMessage {
        fn drop(&mut self) {
            let buffer = unsafe { Box::from_raw(self.buffer.as_ptr()) };
            let mut pool = GLOBAL_BUFFER_POOL.lock();
            if pool.len() < BUFFER_POOL_SIZE_MAX {
                pool.push(buffer);
            }
        }
    }
}
