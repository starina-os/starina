mod boot;
mod cpuvar;
mod csr;
mod entry;
mod hvspace;
mod idle;
mod interrupt;
mod plic;
mod sbi;
mod serial;
mod thread;
mod vmspace;

pub use boot::percpu_init;
pub use cpuvar::CpuVar;
pub use cpuvar::get_cpuvar;
pub use cpuvar::set_cpuvar;
pub use entry::inkernel_syscall_entry;
pub use entry::user_entry;
pub use idle::halt;
pub use idle::idle;
pub use interrupt::INTERRUPT_CONTROLLER;
pub use serial::console_write;
use starina::address::GPAddr;
pub use thread::Thread;
pub use vmspace::PAGE_SIZE;
pub use vmspace::VmSpace;
pub use vmspace::find_free_ram;
pub use vmspace::map_daddr;
pub use vmspace::paddr2vaddr;
pub use vmspace::unmap_daddr;
pub use vmspace::vaddr2paddr;

pub fn hypervisor_test() {
    info!("hypervisor test");

    use core::arch::asm;

    use starina_types::address::GPAddr;
    use starina_types::vmspace::PageProtect;

    // Read hstatus register to check if we are in hypervisor mode.
    let hstatus: u64;
    unsafe {
        asm!("csrr {}, hstatus", out(reg) hstatus);
    }

    info!("hstatus: {:#016x}", hstatus);

    let folio = crate::folio::Folio::alloc(0x1000).unwrap();
    let guest_memory: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(
            paddr2vaddr(folio.paddr()).unwrap().as_mut_ptr(),
            folio.len(),
        )
    };

    let mut hvspace = hvspace::HvSpace::new().unwrap();
    hvspace
        .map(
            GPAddr::new(0x8000_c000),
            folio.paddr(),
            folio.len(),
            PageProtect::READABLE | PageProtect::WRITEABLE | PageProtect::EXECUTABLE,
        )
        .unwrap();

    const BOOT_CODE: &[u32] = &[
        // "wfi"
        0x10500073,
    ];

    // Copy the boot code to the guest memory.
    unsafe {
        core::ptr::copy_nonoverlapping(
            BOOT_CODE.as_ptr(),
            guest_memory.as_mut_ptr() as *mut u32,
            BOOT_CODE.len() as usize / 4,
        );
    };

    // We're ready to run the guest code.

    unsafe {
        info!("hgatp: {:#016x}", hvspace.hgatp());
        asm!(
            "csrw hgatp, {0}",
            in(reg) hvspace.hgatp(),
            options(nostack),
        );
    }

    // Prepare CSRs to go back to VS mode.
    unsafe {
        let mut hstatus: u64;
        asm!("csrr {0}, hstatus", out(reg) hstatus);
        // SPV
        hstatus |= 1 << 7;
        // SPVP
        hstatus |= 1 << 8;
        asm!("csrw hstatus, {0}", in(reg) hstatus);

        let sepc: u64 = 0x8000c000;
        asm!("csrw sepc, {0}", in(reg) sepc);

        // Set the SPP bit to 0 to enter S-mode.
        let mut sstatus: u64;
        asm!("csrr {0}, sstatus", out(reg) sstatus);
        sstatus |= 1 << 8;
        asm!("csrw sstatus, {0}", in(reg) sstatus);

        info!("entering guest mode...");
        asm!("sret");
    }

    // Done!
    panic!("\x1b[1;32mDone!\x1b[0m");
}
