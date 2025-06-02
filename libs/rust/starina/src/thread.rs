use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;

use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::prelude::*;
use crate::syscall;

const THREAD_STACK_SIZE: usize = 1024 * 1024; // 1 MiB

#[allow(dead_code)]
struct Arg {
    sp_top: usize,
    closure: Box<dyn FnOnce() + Send + 'static>,
}

#[cfg(all(target_os = "none", target_arch = "riscv64"))]
#[unsafe(naked)]
extern "C" fn arch_entry() -> ! {
    use core::arch::naked_asm;
    use core::mem::offset_of;

    naked_asm!(
        // a0 points to *mut Arg.
        "ld sp, {sp_offset}(a0)",
        "j {rust_trampoline}",
        sp_offset = const offset_of!(Arg, sp_top),
        rust_trampoline = sym rust_trampoline,
    );
}

#[cfg(not(target_os = "none"))]
extern "C" fn arch_entry() -> ! {
    unimplemented!()
}

#[cfg(target_os = "none")]
fn rust_trampoline(arg: *mut Arg) -> ! {
    let arg = unsafe { Box::from_raw(arg as *mut Arg) };
    (arg.closure)();
    syscall::thread_exit();
}

pub struct Thread {
    _handle: OwnedHandle,
}

impl Thread {
    pub fn spawn<F>(entry: F) -> Result<Self, ErrorCode>
    where
        F: FnOnce() + Send + 'static,
    {
        // FIXME: Skip filling the stack with zeros.
        let stack = vec![0; THREAD_STACK_SIZE];

        let sp_top = stack.as_ptr() as usize + stack.len();
        let arg = Box::into_raw(Box::new(Arg {
            sp_top,
            closure: Box::new(entry),
        }));

        let process = HandleId::from_raw(0); /* current process */
        let handle = syscall::thread_create(process, arch_entry as usize, arg as usize)?;
        Ok(Thread {
            _handle: OwnedHandle::from_raw(handle),
        })
    }
}

impl Handleable for Thread {
    fn handle_id(&self) -> HandleId {
        self._handle.id()
    }
}
