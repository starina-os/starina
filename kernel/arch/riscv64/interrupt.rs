use core::arch::asm;
use core::arch::global_asm;

use crate::arch::__wfi_point;
use crate::arch::riscv64::plic;

global_asm!(include_str!("interrupt.S"));

#[repr(C, packed)]
struct Frame {
    sepc:u64,
    sstatus:u64,
    ra:u64,
    gp:u64,
    tp:u64,
    t0:u64,
    t1:u64,
    t2:u64,
    t3:u64,
    t4:u64,
    t5:u64,
    t6:u64,
    a0:u64,
    a1:u64,
    a2:u64,
    a3:u64,
    a4:u64,
    a5:u64,
    a6:u64,
    a7:u64,
    s0:u64,
    s1:u64,
    s2:u64,
    s3:u64,
    s4:u64,
    s5:u64,
    s6:u64,
    s7:u64,
    s8:u64,
    s9:u64,
    s10:u64,
    s11:u64,
}

#[no_mangle]
extern "C" fn interrupt_handler(frame: *mut Frame) {
    let scause: u64;
    let sepc: u64;
    let stval: u64;
    unsafe {
        asm!("csrr {}, scause", out(reg) scause);
        asm!("csrr {}, sepc", out(reg) sepc);
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

    unsafe {
        if sepc == &__wfi_point as *const _ as u64 {
            // Skip WFI instruction.
            (*frame).sepc += 4;
        }
    }

    if (is_intr, code) == (true, 9) {
        plic::handle_interrupt();
        return;
    }

    panic!(
        "interrupt_handler: {} (scause={:#x}), sepc: {:#x}, stval: {:#x}",
        scause_str, scause, sepc, stval
    );
}
