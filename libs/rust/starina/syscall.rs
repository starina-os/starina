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

#[cfg(feature = "in-kernel")]
unsafe extern "Rust" {
    safe static INKERNEL_SYSCALL_TABLE: InKernelSyscallTable;
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

#[cfg(feature = "in-kernel")]
pub fn console_write(s: &[u8]) {
    (INKERNEL_SYSCALL_TABLE.console_write)(s);
}

#[cfg(feature = "in-kernel")]
pub fn thread_yield() {
    (INKERNEL_SYSCALL_TABLE.thread_yield)();
}

#[cfg(feature = "in-kernel")]
pub fn poll_create() -> Result<HandleId, ErrorCode> {
    (INKERNEL_SYSCALL_TABLE.poll_create)()
}

#[cfg(feature = "in-kernel")]
pub fn poll_add(poll: HandleId, object: HandleId, interests: Readiness) -> Result<(), ErrorCode> {
    (INKERNEL_SYSCALL_TABLE.poll_add)(poll, object, interests)
}

#[cfg(feature = "in-kernel")]
pub fn poll_wait(poll: HandleId) -> Result<(HandleId, Readiness), ErrorCode> {
    (INKERNEL_SYSCALL_TABLE.poll_wait)(poll)
}

#[cfg(feature = "in-kernel")]
pub fn channel_send(
    ch: HandleId,
    msginfo: MessageInfo,
    data: *const u8,
    handles: *const HandleId,
) -> Result<(), ErrorCode> {
    (INKERNEL_SYSCALL_TABLE.channel_send)(ch, msginfo, data, handles)
}

#[cfg(feature = "in-kernel")]
pub fn channel_recv(
    ch: HandleId,
    data: *mut u8,
    handles: *mut HandleId,
) -> Result<MessageInfo, ErrorCode> {
    (INKERNEL_SYSCALL_TABLE.channel_recv)(ch, data, handles)
}

#[cfg(feature = "in-kernel")]
pub fn handle_close(handle: HandleId) -> Result<(), ErrorCode> {
    (INKERNEL_SYSCALL_TABLE.handle_close)(handle)
}
