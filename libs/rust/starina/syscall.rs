pub struct InKernelSyscallTable {
    pub console_write: fn(&[u8]),
}

#[cfg(feature = "in-kernel")]
unsafe extern "Rust" {
    safe static INKERNEL_SYSCALL_TABLE: InKernelSyscallTable;
}

#[cfg(feature = "in-kernel")]
pub fn console_write(s: &[u8]) {
    (INKERNEL_SYSCALL_TABLE.console_write)(s);
}
