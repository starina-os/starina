use starina::syscall::InKernelSyscallTable;

#[no_mangle]
static INKERNEL_SYSCALL_TABLE: InKernelSyscallTable = InKernelSyscallTable {
    console_write: crate::arch::console_write,
};
