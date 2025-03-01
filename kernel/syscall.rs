use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::poll::Readiness;
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

#[derive(Debug, Clone, Copy)]
pub struct RetVal(isize);

impl RetVal {
    pub const fn new(value: isize) -> RetVal {
        RetVal(value)
    }

    pub fn as_isize(&self) -> isize {
        self.0
    }
}

impl From<Result<(HandleId, Readiness), ErrorCode>> for RetVal {
    fn from(value: Result<(HandleId, Readiness), ErrorCode>) -> Self {
        todo!()
    }
}

pub extern "C" fn syscall_handler(
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
) -> ! {
    trace!(
        "syscall_handler: a0={:x}, a1={:x}, a2={:x}, a3={:x}, a4={:x}, a5={:x}",
        a0, a1, a2, a3, a4, a5
    );
    switch_thread();
}
