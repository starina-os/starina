use std::io::Write;

pub fn percpu_init() {
    todo!()
}

pub fn halt() -> ! {
    panic!("halted");
}

pub fn console_write(s: &[u8]) {
    std::io::stdout().write_all(s).unwrap();
}

// #[naked]
pub extern "C" fn enter_kernelland(
    _a0: isize,
    _a1: isize,
    _a2: isize,
    _a3: isize,
    _a4: isize,
    _a5: isize,
) -> isize {
    todo!()
}

pub fn enter_userland(thread: *mut crate::arch::Thread) -> ! {
    todo!()
}

pub fn idle() -> ! {
    todo!();
}

pub struct Thread {}

impl Thread {
    pub fn new_inkernel(pc: usize, arg: usize) -> Thread {
        todo!()
    }

    pub fn new_idle() -> Thread {
        todo!()
    }
}

pub const NUM_CPUS_MAX: usize = 1;

pub struct CpuVar {}

impl CpuVar {
    pub fn new(idle_thread: &crate::refcount::SharedRef<crate::thread::Thread>) -> Self {
        todo!()
    }
}

pub fn set_cpuvar(cpuvar: *mut crate::cpuvar::CpuVar) {
    todo!()
}

pub fn get_cpuvar() -> &'static crate::cpuvar::CpuVar {
    todo!()
}
