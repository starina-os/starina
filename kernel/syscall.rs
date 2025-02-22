use starina::syscall::DirectSyscallTable;

#[no_mangle]
static DIRECT_SYSCALL_TABLE: DirectSyscallTable = DirectSyscallTable {
    console_write: crate::arch::console_write,
};
