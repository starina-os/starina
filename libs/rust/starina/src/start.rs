use core::slice;
use core::str;

use crate::environ::Environ;
use crate::syscall;
use crate::syscall::VsyscallPage;
use crate::tls;

pub extern "C" fn start(vsyscall: *const VsyscallPage) -> ! {
    let vsyscall = unsafe { &*vsyscall };

    let name_slice = unsafe { slice::from_raw_parts(vsyscall.name, vsyscall.name_len) };
    let name = str::from_utf8(name_slice).unwrap();
    tls::init_thread_local(name);

    crate::log::init();

    let env_json = unsafe { slice::from_raw_parts(vsyscall.environ_ptr, vsyscall.environ_len) };
    let environ = unsafe { Environ::from_raw(env_json) };
    (vsyscall.main)(environ);
    syscall::thread_exit();
}
