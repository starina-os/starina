use core::arch::asm;

use super::plic;
use crate::syscall::syscall_handler;
use crate::thread::Thread;

pub extern "C" fn interrupt_handler() -> ! {
    let cpuvar = super::get_cpuvar();

    let scause: u64;
    let stval: u64;
    unsafe {
        asm!("csrr {}, scause", out(reg) scause);
        asm!("csrr {}, stval", out(reg) stval);
    }

    let is_intr = scause & (1 << 63) != 0;
    let code = scause & !(1 << 63);
    let scause_str = match (is_intr, code) {
        (true, 0) => "user software interrupt",
        (true, 1) => "supervisor software interrupt",
        (true, 2) => "hypervisor software interrupt",
        (true, 3) => "machine software interrupt",
        (true, 4) => "user timer interrupt",
        (true, 5) => "supervisor timer interrupt",
        (true, 6) => "hypervisor timer interrupt",
        (true, 7) => "machine timer interrupt",
        (true, 8) => "user external interrupt",
        (true, 9) => "supervisor external interrupt",
        (true, 10) => "hypervisor external interrupt",
        (true, 11) => "machine external interrupt",
        (false, 0) => "instruction address misaligned",
        (false, 1) => "instruction access fault",
        (false, 2) => "illegal instruction",
        (false, 3) => "breakpoint",
        (false, 4) => "load address misaligned",
        (false, 5) => "load access fault",
        (false, 6) => "store/AMO address misaligned",
        (false, 7) => "store/AMO access fault",
        (false, 8) => "environment call from U-mode",
        (false, 9) => "environment call from S-mode",
        (false, 10) => "reserved",
        (false, 11) => "environment call from M-mode",
        (false, 12) => "instruction page fault",
        (false, 13) => "load page fault",
        (false, 15) => "store/AMO page fault",
        _ => "unknown",
    };

    let sepc = unsafe { (*cpuvar.arch.context).sepc } as u64;

    trace!(
        "interrupt: {} (scause={:#x}), sepc: {:#x}, stval: {:#x}",
        scause_str,
        scause,
        sepc,
        stval
    );

    if (is_intr, code) == (true, 9) {
        plic::handle_interrupt();
    } else if (is_intr, code) == (false, 8) {
        unsafe {
            // Skip ecall instruction
            (*cpuvar.arch.context).sepc += 4 as usize;
        }

        let a0 = unsafe { (*cpuvar.arch.context).a0 } as isize;
        let a1 = unsafe { (*cpuvar.arch.context).a1 } as isize;
        let a2 = unsafe { (*cpuvar.arch.context).a2 } as isize;
        let a3 = unsafe { (*cpuvar.arch.context).a3 } as isize;
        let a4 = unsafe { (*cpuvar.arch.context).a4 } as isize;
        let a5 = unsafe { (*cpuvar.arch.context).a5 } as isize;
        let ret = syscall_handler(a0, a1, a2, a3, a4, a5);
        unsafe {
            (*cpuvar.arch.context).a0 = ret as usize;
        }
    } else {
        panic!("unhandled intrrupt");
    }

    Thread::switch();
}
