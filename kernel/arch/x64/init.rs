use super::gdt;
use super::idt;
use super::mptable;
use super::pic;
use super::tss;
use super::CpuId;

pub fn early_init(cpu_id: CpuId) {
    // SeaBIOS disables line wrapping and cuts off the output. Enable it again
    // to preserve the default terminal behavior you would expect.
    println!("\x1b[?7h");

    const CR4_FSGSBASE: u64 = 1 << 16;
    unsafe {
        let mut cr4: u64;
        core::arch::asm!("mov rax, cr4", out("rax") cr4);
        cr4 |= CR4_FSGSBASE;
        core::arch::asm!("mov cr4, rax", in("rax") cr4);
    }

    pic::init();
    gdt::init();
    tss::init();
    idt::init();
    mptable::init();
}

pub fn init(cpu_id: CpuId, device_tree: Option<&crate::device_tree::DeviceTree>) {}
