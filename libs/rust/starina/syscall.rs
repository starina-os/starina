pub struct InKernelSyscallTable {
    pub console_write: fn(&[u8]),
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
