use crate::address::DAddr;
use crate::address::VAddr;
use crate::error::ErrorCode;
use crate::handle::HandleId;
use crate::poll::Readiness;

pub const SYS_CONSOLE_WRITE: u8 = 0;
pub const SYS_POLL_CREATE: u8 = 1;
pub const SYS_POLL_ADD: u8 = 2;
pub const SYS_POLL_WAIT: u8 = 3;
pub const SYS_CHANNEL_SEND: u8 = 4;
pub const SYS_CHANNEL_RECV: u8 = 5;
pub const SYS_HANDLE_CLOSE: u8 = 7;
pub const SYS_FOLIO_CREATE: u8 = 8;
pub const SYS_FOLIO_PADDR: u8 = 9;
pub const SYS_FOLIO_CREATE_FIXED: u8 = 10;
pub const SYS_VMSPACE_MAP: u8 = 11;
pub const SYS_BUSIO_MAP: u8 = 12;
pub const SYS_FOLIO_DADDR: u8 = 13;
#[repr(C)]
pub struct VsyscallPage {
    pub environ_ptr: *const u8,
    pub environ_len: usize,
}

/// SAFETY: VsyscallPage is pre-allocated, the same across threads, and immutable.
unsafe impl Send for VsyscallPage {}

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

impl From<VAddr> for RetVal {
    fn from(value: VAddr) -> Self {
        RetVal(value.as_usize() as isize)
    }
}

impl From<DAddr> for RetVal {
    fn from(value: DAddr) -> Self {
        RetVal(value.as_usize() as isize)
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
