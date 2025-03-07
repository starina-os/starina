use crate::error::ErrorCode;
use crate::handle::HandleId;
use crate::message::MessageInfo;
use crate::poll::Readiness;

pub struct InKernelSyscallTable {
    pub console_write: fn(&[u8]),
    pub poll_create: fn() -> Result<HandleId, ErrorCode>,
    pub poll_add: fn(
        HandleId, /* poll */
        HandleId, /* object */
        Readiness,
    ) -> Result<(), ErrorCode>,
    pub poll_wait: fn(HandleId) -> Result<(HandleId, Readiness), ErrorCode>,
    pub channel_send: fn(HandleId, *const u8, *const HandleId) -> Result<(), ErrorCode>,
    pub channel_recv: fn(HandleId, *mut u8, *mut HandleId) -> Result<MessageInfo, ErrorCode>,
    pub thread_yield: fn(),
}

#[cfg(feature = "in-kernel")]
unsafe extern "Rust" {
    safe static INKERNEL_SYSCALL_TABLE: InKernelSyscallTable;
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
    handle: HandleId,
    data: *const u8,
    handles: *const HandleId,
) -> Result<(), ErrorCode> {
    (INKERNEL_SYSCALL_TABLE.channel_send)(handle, data, handles)
}

#[cfg(feature = "in-kernel")]
pub fn channel_recv(
    handle: HandleId,
    data: *mut u8,
    handles: *mut HandleId,
) -> Result<MessageInfo, ErrorCode> {
    (INKERNEL_SYSCALL_TABLE.channel_recv)(handle, data, handles)
}
