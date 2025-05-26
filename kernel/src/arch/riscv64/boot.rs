use core::arch::asm;
use core::arch::naked_asm;


use super::get_cpuvar;
use super::plic::use_plic;
use crate::BootInfo;
use crate::arch::riscv64::csr::StvecMode;
use crate::arch::riscv64::csr::write_stvec;
use crate::arch::riscv64::entry::trap_entry;
use crate::cpuvar::CpuId;

// The kernel entrypoint for RISC-V machines. We expect Linux's RISC-V boot
// requirements:
//
//   - a0: THe hartid of this CPU.
//   - a1: The address of the device tree blob.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
#[unsafe(naked)]
unsafe extern "C" fn riscv64_boot(hartid: u64, dtb: *const u8) -> ! {
    unsafe {
        naked_asm!(
            // Note: Don't modify a0, a1 registers here: they are used as arguments to
            //       rust_boot.
            "mv ra, zero",
            "mv fp, zero",
            "la sp, __boot_stack_top",
            "j {rust_boot}",
            rust_boot = sym rust_boot,
        );
    }
}

unsafe extern "C" {
    static __bss: u8;
    static __bss_end: u8;
}

unsafe extern "C" fn rust_boot(hartid: u64, dtb: *const u8) -> ! {
    // Clear bss.
    unsafe {
        let bss_start = &raw const __bss as usize;
        let bss_end = &raw const __bss_end as usize;
        core::ptr::write_bytes(bss_start as *mut u8, 0, bss_end - bss_start);
    }

    let cpu_id = CpuId::new(hartid.try_into().unwrap());
    crate::boot(BootInfo { cpu_id, dtb });
}

pub fn percpu_init() {
    unsafe {
        asm!("csrw sscratch, tp");
    }

    unsafe {
        write_stvec(trap_entry as *const () as usize, StvecMode::Direct);

        let mut sie: u64;
        asm!("csrr {}, sie", out(reg) sie);
        sie |= 1 << 1; // SSIE: supervisor-level software interrupts
        sie |= 1 << 5; // STIE: supervisor-level timer interrupts
        sie |= 1 << 9; // SEIE: supervisor-level external interrupts
        asm!("csrw sie, {}", in(reg) sie);
    }

    super::vcpu::init();

    use_plic(|plic| {
        plic.init_per_cpu(get_cpuvar().cpu_id);
    });
}
