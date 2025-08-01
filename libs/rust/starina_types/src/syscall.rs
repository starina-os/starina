use crate::address::PAddr;
use crate::address::VAddr;
use crate::environ::Environ;
use crate::error::ErrorCode;
use crate::handle::HandleId;
use crate::poll::Readiness;
use crate::timer::MonotonicTime;

pub const SYS_LOG_WRITE: u8 = 0;
pub const SYS_HANDLE_CLOSE: u8 = 1;
pub const SYS_CHANNEL_CREATE: u8 = 2;
pub const SYS_CHANNEL_SEND: u8 = 3;
pub const SYS_CHANNEL_RECV: u8 = 4;
pub const SYS_POLL_CREATE: u8 = 5;
pub const SYS_POLL_ADD: u8 = 6;
pub const SYS_POLL_UPDATE: u8 = 7;
pub const SYS_POLL_REMOVE: u8 = 8;
pub const SYS_POLL_WAIT: u8 = 9;
pub const SYS_POLL_TRY_WAIT: u8 = 10;
pub const SYS_FOLIO_ALLOC: u8 = 11;
pub const SYS_FOLIO_PIN: u8 = 12;
pub const SYS_FOLIO_PADDR: u8 = 13;
pub const SYS_VMSPACE_MAP: u8 = 14;
pub const SYS_INTERRUPT_CREATE: u8 = 15;
pub const SYS_INTERRUPT_ACK: u8 = 16;
pub const SYS_THREAD_EXIT: u8 = 17;
pub const SYS_HVSPACE_CREATE: u8 = 18;
pub const SYS_HVSPACE_MAP: u8 = 19;
pub const SYS_VCPU_CREATE: u8 = 20;
pub const SYS_VCPU_RUN: u8 = 21;
pub const SYS_THREAD_SPAWN: u8 = 22;
pub const SYS_TIMER_CREATE: u8 = 23;
pub const SYS_TIMER_SET: u8 = 24;
pub const SYS_TIMER_NOW: u8 = 25;
pub const SYS_LOG_READ: u8 = 26;

#[repr(C)]
pub struct VsyscallPage {
    pub environ_ptr: *const u8,
    pub environ_len: usize,
    pub main: fn(environ: Environ),
    pub name: *const u8,
    pub name_len: usize,
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

impl From<PAddr> for RetVal {
    fn from(value: PAddr) -> Self {
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
            Readiness::from_raw(readiness as u8),
        )
    }
}

impl From<RetVal> for HandleId {
    fn from(value: RetVal) -> Self {
        HandleId::from_raw(value.0 as i32)
    }
}

impl From<MonotonicTime> for RetVal {
    fn from(value: MonotonicTime) -> Self {
        RetVal(value.as_nanos() as isize)
    }
}

impl From<RetVal> for MonotonicTime {
    fn from(value: RetVal) -> Self {
        MonotonicTime::from_nanos(value.0 as u64)
    }
}
