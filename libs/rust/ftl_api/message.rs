use core::mem;
use core::ptr;

use ftl_types::handle::HandleId;
use ftl_types::message::MessageInfo;
use ftl_types::message::MESSAGE_DATA_MAX_LEN;
use ftl_types::message::MESSAGE_HANDLES_MAX_COUNT;

#[repr(C, align(16))] // Align to 16 bytes for SIMD instructions.
pub struct MessageBuffer {
    pub data: [u8; MESSAGE_DATA_MAX_LEN],
    pub handles: [HandleId; MESSAGE_HANDLES_MAX_COUNT],
}

impl MessageBuffer {
    pub fn new() -> Self {
        // TODO: Avoid zeroing the buffer because it's not necessary.
        Self {
            data: [0; MESSAGE_DATA_MAX_LEN],
            handles: [HandleId::from_raw(0); MESSAGE_HANDLES_MAX_COUNT],
        }
    }

    pub(crate) fn write<T: MessageBody>(&mut self, msg: T) {
        let dst = self as *mut _ as *mut T;
        let src = &msg as *const T;

        // Use ptr::copy_nonoverlapping to avoid calling destructors (i.e. avoid
        // freeing moved handles) and to avoid unnecessary length checks.
        //
        // SAFETY: Let's check the requirements for ptr::copy_nonoverlapping one
        //         by one:
        //
        // > src must be valid for reads of count * size_of::<T>() bytes.
        //
        // True because msg is a valid reference to a single T.
        //
        // > dst must be valid for writes of count * size_of::<T>() bytes.
        //
        // We assume size_of::<T>() <= size_of::<MessageBuffer> holds. It is
        // guaranteed IDL stub generator.
        //
        // > Both src and dst must be properly aligned.
        //
        // True because MessageBuffer is aligned to 16 bytes through #[repr] and
        // IDL stub generator guarantees that it's big enough for all field types
        // in T.
        //
        // > The region of memory beginning at src with a size of
        // > count * size_of::<T>() bytes must not overlap with the region of
        // > memory beginning at dst with the same size.
        //
        // True because self.buffer is unique and we have an exclusive acces
        //  (&mut self) to it.
        unsafe {
            ptr::copy_nonoverlapping::<T>(src, dst, 1);
        }

        // Don't call destructors on handles transferred to this buffer.
        mem::forget(msg);
    }
}

/// Invariant: size_of::<MessageBuffer> >= size_of::<T>().
pub trait MessageBody {
    const MSGINFO: MessageInfo;
    type Reader<'a>: 'a;
    fn deserialize<'a>(buffer: &'a MessageBuffer) -> Self::Reader<'a>;
}

#[repr(C)]
pub struct PingPongMessage {
    pub value: isize,
}

impl MessageBody for PingPongMessage {
    const MSGINFO: MessageInfo = MessageInfo::from_raw(4);
    type Reader<'a> = PingPongMessageReader<'a>;

    fn deserialize<'a>(buffer: &'a MessageBuffer) -> Self::Reader<'a> {
        PingPongMessageReader { buffer }
    }
}

pub struct PingPongMessageReader<'a> {
    buffer: &'a MessageBuffer,
}

impl<'a> PingPongMessageReader<'a> {
    pub fn value(&self) -> isize {
        unsafe { (*(self.buffer as *const _ as *const PingPongMessage)).value }
    }
}
