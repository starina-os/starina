pub struct DirectSyscallTable {
    pub console_write: fn(&[u8]),
}

#[cfg(feature = "kernel")]
extern "Rust" {
    #[no_mangle]
    static DIRECT_SYSCALL_TABLE: DirectSyscallTable;
}

#[cfg(feature = "kernel")]
pub fn console_write(s: &[u8]) {
    unsafe {
        (DIRECT_SYSCALL_TABLE.console_write)(s);
    }
}
