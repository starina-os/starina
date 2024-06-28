use ftl_types::syscall::VsyscallPage;

use crate::allocator;
use crate::syscall::set_vsyscall;

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
pub unsafe fn init_internal(vsyscall_page: *const VsyscallPage) {
    // SAFETY: Vsyscall page will be always available at the same
    //         address.
    let vsyscall = unsafe { &*vsyscall_page };

    set_vsyscall(vsyscall);
    allocator::init();
}
