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

/// Initializes the FTL API.
///
/// # Warning
///
/// Do not use this function. This is intended to be called by `ftl_api_macros`
/// only.
///
/// # Safety
///
/// Make sure you call this function only once. If you call this function
/// may accidentally overwrite in-use memory objects by reinitializing the
/// allocator!
pub unsafe fn init_internal(vsyscall_page: *const VsyscallPage) -> Environ {
    // SAFETY: Vsyscall page will be always available at the same
    //         address.
    let vsyscall = unsafe { &*vsyscall_page };

    syscall::set_vsyscall(vsyscall);
    allocator::init();

    let mut env = parse_environ(vsyscall);

    let vmspace = env.take_vmspace("vmspace").unwrap();
    APP_VMSPACE.lock().replace(vmspace);

    env
}
