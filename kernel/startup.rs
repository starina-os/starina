use starina::syscall::VsyscallPage;

use crate::process::KERNEL_PROCESS;
use crate::scheduler::GLOBAL_SCHEDULER;
use crate::thread::Thread;

pub struct InKernelApp {
    name: &'static str,
    main: fn(vsyscall: *const VsyscallPage),
}

const INKERNEL_APPS: &[InKernelApp] = &[InKernelApp {
    name: "virtio_net",
    main: virtio_net::autogen::app_main,
}];

pub fn load_inkernel_apps() {
    GLOBAL_SCHEDULER.push(Thread::new_inkernel(virtio_net::autogen::app_main as usize, 0).unwrap());
}
