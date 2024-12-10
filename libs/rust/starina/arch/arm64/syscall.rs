use core::arch::asm;

use starina_types::error::ErrorCode;
use starina_types::syscall::SyscallNumber;
use starina_types::syscall::VsyscallPage;

#[inline]
fn vsyscall_page() -> &'static VsyscallPage {
    let ptr: *const VsyscallPage;
    unsafe {
        asm!(
            "mrs {ptr}, tpidr_el0",
            ptr = out(reg) ptr,
            options(nostack),
            options(nomem),
        );
    }

    // SAFETY: The vsyscall page always exists.
    unsafe { &*ptr }
}

pub fn syscall(
    n: SyscallNumber,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
) -> Result<isize, ErrorCode> {
    let vsyscall = vsyscall_page();
    let mut ret: isize;
    unsafe {
        asm!(
            "blr {entry}",
            entry = in(reg) vsyscall.entry,
            inout("x0") a0 as isize => ret,
            in("x1") a1 as isize,
            in("x2") a2 as isize,
            in("x3") a3 as isize,
            in("x4") a4 as isize,
            in("x5") n as isize,
            options(nostack),
        );
    }

    if ret < 0 {
        let err = unsafe { core::mem::transmute::<i8, ErrorCode>(ret as i8) };
        return Err(err);
    }

    Ok(ret)
}
