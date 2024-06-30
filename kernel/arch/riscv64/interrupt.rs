use core::arch::asm;

// The interrupt/exception/system call handler entry point. `stvec` is set to
// this address.
//
// This function address must be aligned to 4 bytes not to accidentally set
// MODE field in stvec.
#[naked]
#[repr(align(4))] // handle address in stvec must be aligned
pub unsafe extern "C" fn switch_to_kernel() -> ! {
    asm!(
        // tp points to the current thread's context
        r#"
        call {interrupt_handler}
        "#
        ,
        interrupt_handler = sym interrupt_handler,
        options(noreturn),
    )
}

extern "C" fn interrupt_handler() -> ! {
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

    panic!(
        "interrupt_handler: {} (scause={:#x}), sepc: {:#x}, stval: {:#x}",
        scause_str, scause, sepc, stval
    );
}
