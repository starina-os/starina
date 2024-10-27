use core::arch::asm;
use core::arch::naked_asm;
use core::mem::offset_of;

use super::gic_v2;
use crate::arch::arm64_exception_vector;

#[no_mangle]
extern "C" fn arm64_handle_idle_exception() {
    panic!("unhandled exception (idle)");
    // TODO: restore vbar_el1
}

extern "C" fn do_handle_interrupt() -> ! {
    gic_v2::handle_interrupt();
    crate::thread::Thread::switch();
}

#[no_mangle]
#[naked]
unsafe extern "C" fn arm64_handle_idle_interrupt() {
    naked_asm!(
        r#"
            // Disable interrupts
            msr daifset, #2

            // Restore vbar_el1
            ldr x0, ={arm64_exception_vector}
            msr vbar_el1, x0

            // Get the per-CPU stack
            mrs x0, tpidr_el1
            ldr x1, [x0, {kernel_sp_offset}]
            mov sp, x1

            b {do_handle_interrupt}
        "#,
        arm64_exception_vector = sym arm64_exception_vector,
        kernel_sp_offset = const offset_of!(crate::arch::CpuVar, kernel_sp),
        do_handle_interrupt = sym do_handle_interrupt,
    );
}

pub fn idle() -> ! {
    trace!("idle");

    extern "C" {
        static arm64_idle_exception_vector: [u8; 128 * 16];
    }

    unsafe {
        asm!("msr vbar_el1, {}", in(reg) &arm64_idle_exception_vector as *const _ as u64);
        asm!("msr daifclr, #2");
        loop {
            asm!("wfi");
        }
    }
}
