use crate::error::ErrorCode;
use crate::handle::HandleId;
use crate::message::MessageInfo;
use crate::poll::Readiness;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SyscallNumber {
    ConsoleWrite = 0,
    PollCreate = 1,
    PollAdd = 2,
    PollWait = 3,
    ChannelSend = 4,
    ChannelRecv = 5,
    ThreadYield = 6,
    HandleClose = 7,
}

#[repr(C)]
pub struct VsyscallPage {
    pub environ_ptr: *const u8,
    pub environ_len: usize,
}

pub struct InKernelSyscallTable {
    pub console_write: fn(&[u8]),
    pub poll_create: fn() -> Result<HandleId, ErrorCode>,
    pub poll_add: fn(
        HandleId, /* poll */
        HandleId, /* object */
        Readiness,
    ) -> Result<(), ErrorCode>,
    pub poll_wait: fn(HandleId) -> Result<(HandleId, Readiness), ErrorCode>,
    pub channel_send:
        fn(HandleId, MessageInfo, *const u8, *const HandleId) -> Result<(), ErrorCode>,
    pub channel_recv: fn(HandleId, *mut u8, *mut HandleId) -> Result<MessageInfo, ErrorCode>,
    pub thread_yield: fn(),
    pub handle_close: fn(HandleId) -> Result<(), ErrorCode>,
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct RetVal(isize);

impl RetVal {
    pub const fn new(value: isize) -> RetVal {
        RetVal(value)
    }

    pub fn as_isize(&self) -> isize {
        self.0
    }
}

impl<T> From<Result<T, ErrorCode>> for RetVal
where
    T: Into<RetVal>,
{
    fn from(value: Result<T, ErrorCode>) -> Self {
        match value {
            Ok(value) => value.into(),
            Err(err) => RetVal(err as isize),
        }
    }
}

impl From<(HandleId, Readiness)> for RetVal {
    fn from(value: (HandleId, Readiness)) -> Self {
        let handle_raw = value.0.as_raw() as isize;
        assert!(handle_raw < 0x10000);
        let readiness = value.1.as_isize();
        RetVal((readiness << 24) | handle_raw)
    }
}

impl From<ErrorCode> for RetVal {
    fn from(value: ErrorCode) -> Self {
        RetVal(value as isize)
    }
}

impl<T> From<RetVal> for Result<T, ErrorCode>
where
    T: From<RetVal>,
{
    fn from(value: RetVal) -> Self {
        if value.0 >= 0 {
            let value = value.into();
            Ok(value)
        } else {
            let code = unsafe { core::mem::transmute_copy(&value.0) };
            Err(code)
        }
    }
}

impl From<RetVal> for (HandleId, Readiness) {
    fn from(value: RetVal) -> Self {
        let handle_raw = value.0 & 0x00ff_ffff;
        let readiness = value.0 >> 24;
        (
            HandleId::from_raw(handle_raw as i32),
            Readiness::from_raw(readiness as i8),
        )
    }
}
