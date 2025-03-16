pub use starina_types::syscall::*;

#[cfg(feature = "in-kernel")]
pub use self::in_kernel::*;

pub mod in_kernel {
    use starina_types::address::PAddr;
    use starina_types::address::VAddr;
    use starina_types::error::ErrorCode;
    use starina_types::handle::HandleId;
    use starina_types::message::MessageInfo;
    use starina_types::poll::Readiness;
    use starina_types::vmspace::PageProtect;

    use super::*;

    unsafe extern "Rust" {
        safe static INKERNEL_SYSCALL_TABLE: InKernelSyscallTable;
    }

    pub fn console_write(s: &[u8]) {
        (INKERNEL_SYSCALL_TABLE.console_write)(s);
    }

    pub fn thread_yield() {
        (INKERNEL_SYSCALL_TABLE.thread_yield)();
    }

    pub fn poll_create() -> Result<HandleId, ErrorCode> {
        (INKERNEL_SYSCALL_TABLE.poll_create)()
    }

    pub fn poll_add(
        poll: HandleId,
        object: HandleId,
        interests: Readiness,
    ) -> Result<(), ErrorCode> {
        (INKERNEL_SYSCALL_TABLE.poll_add)(poll, object, interests)
    }

    pub fn poll_wait(poll: HandleId) -> Result<(HandleId, Readiness), ErrorCode> {
        (INKERNEL_SYSCALL_TABLE.poll_wait)(poll)
    }

    pub fn channel_send(
        ch: HandleId,
        msginfo: MessageInfo,
        data: *const u8,
        handles: *const HandleId,
    ) -> Result<(), ErrorCode> {
        (INKERNEL_SYSCALL_TABLE.channel_send)(ch, msginfo, data, handles)
    }

    pub fn channel_recv(
        ch: HandleId,
        data: *mut u8,
        handles: *mut HandleId,
    ) -> Result<MessageInfo, ErrorCode> {
        (INKERNEL_SYSCALL_TABLE.channel_recv)(ch, data, handles)
    }

    pub fn handle_close(handle: HandleId) -> Result<(), ErrorCode> {
        (INKERNEL_SYSCALL_TABLE.handle_close)(handle)
    }

    pub fn folio_create(len: usize) -> Result<HandleId, ErrorCode> {
        todo!()
    }

    pub fn folio_create_fixed(paddr: PAddr, len: usize) -> Result<HandleId, ErrorCode> {
        todo!()
    }

    pub fn folio_paddr(handle: HandleId) -> Result<usize, ErrorCode> {
        todo!()
    }

    pub fn vmspace_map(
        handle: HandleId,
        len: usize,
        folio: HandleId,
        prot: PageProtect,
    ) -> Result<VAddr, ErrorCode> {
        todo!()
    }
}
