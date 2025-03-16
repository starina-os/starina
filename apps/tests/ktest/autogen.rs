use starina::syscall::VsyscallPage;

use crate::App;

// TODO: Remove this.
pub fn app_main(vsyscall: *const VsyscallPage) {
    starina::eventloop::app_loop::<(), App>(vsyscall);
}
