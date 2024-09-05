use ftl_types::handle::HandleId;
use ftl_types::syscall::VsyscallPage;

use crate::allocator;
use crate::environ::Environ;
use crate::syscall;
use crate::vmspace::VmSpace;

// TODO: Avoid Mutex.
static APP_VMSPACE: spin::Mutex<Option<VmSpace>> = spin::Mutex::new(None);

pub fn app_vmspace_handle() -> HandleId {
    APP_VMSPACE.lock().as_ref().unwrap().handle().id()
}

fn parse_environ(vsyscall: &VsyscallPage) -> Environ {
    let env_bytes =
        unsafe { ::core::slice::from_raw_parts((*vsyscall).environ_ptr, (*vsyscall).environ_len) };
    let env_str = ::core::str::from_utf8(env_bytes).unwrap();

    Environ::parse(env_str)
}

extern "Rust" {
    fn main(env: Environ);
}

#[no_mangle]
pub unsafe extern "C" fn start_rust(vsyscall_page: *const VsyscallPage) {
    // SAFETY: Vsyscall page will be always available at the same
    //         address.
    let vsyscall = unsafe { &*vsyscall_page };

    syscall::set_vsyscall(vsyscall);
    allocator::init();

    let mut env = parse_environ(vsyscall);

    let vmspace = env.take_vmspace("vmspace").unwrap();
    APP_VMSPACE.lock().replace(vmspace);

    main(env);
}
