use core::mem::MaybeUninit;

#[repr(C)]
pub struct MessageBuffer {
    pub data: [u8; 4095],
    _pad: u8,
}

impl MessageBuffer {
    pub const fn new() -> MessageBuffer {
        // SAFETY: The buffer will be initialized by IPC stubs when sending,
        //         a message, and by the kernel when receiving a message.
        unsafe {
            // Use MaybeUninit to memset the buffer to zero unnecessarily and
            // just prepare an uninitialized buffer on the stack.
            let uninit = MaybeUninit::<MessageBuffer>::uninit();
            uninit.assume_init()
        }
    }
}
