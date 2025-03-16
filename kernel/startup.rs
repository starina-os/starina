use starina::syscall::VsyscallPage;
use starina_types::spec::AppSpec;

use crate::process::KERNEL_PROCESS;
use crate::scheduler::GLOBAL_SCHEDULER;
use crate::thread::Thread;

struct InKernelApp {
    name: &'static str,
    main: fn(vsyscall: *const VsyscallPage),
    spec: AppSpec,
}

const INKERNEL_APPS: &[InKernelApp] = &[InKernelApp {
    name: "virtio_net",
    main: virtio_net::autogen::app_main,
    spec: virtio_net::autogen::APP_SPEC,
}];

pub fn load_inkernel_apps() {
    GLOBAL_SCHEDULER.push(Thread::new_inkernel(virtio_net::autogen::app_main as usize, 0).unwrap());
}
