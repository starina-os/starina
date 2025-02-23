use starina::syscall::InKernelSyscallTable;

use crate::arch::enter_kernelland;
use crate::thread::switch_thread;

#[unsafe(no_mangle)]
static INKERNEL_SYSCALL_TABLE: InKernelSyscallTable = InKernelSyscallTable {
    console_write: crate::arch::console_write,
    thread_yield: thread_yield_trampoline,
};

fn thread_yield_trampoline() {
    enter_kernelland(123, 0, 0, 0, 0, 0);
}

pub fn syscall_handler(a0: usize) {
    println!("syscall_handler: a0={}", a0);
    switch_thread();
}
