use starina::address::GPAddr;
use starina::prelude::*;
use vm_fdt::FdtWriter;

const GUEST_RAM_START: u64 = 0x80000000; // Standard QEMU virt machine RAM start
const GUEST_RAM_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB RAM, adjust as needed
const GUEST_HART_ID_BASE: u32 = 0; // Starting Hart ID
const GUEST_TIMEBASE_FREQ: u32 = 10000000; // QEMU virt default timer frequency (10 MHz)
const GUEST_MMU_TYPE: &str = "riscv,sv48"; // Common MMU type for RV64

fn plic_size(num_cpus: usize) -> u64 {
    0x200000 * (num_cpus as u64 * 0x1000)
}

pub fn build_fdt(
    num_cpus: usize,
    plic_base: GPAddr,
    virtio_mmios: &[GPAddr],
) -> Result<Vec<u8>, vm_fdt::Error> {
    let mut fdt = FdtWriter::new()?;

    // Root node: Defines global properties
    let root_node = fdt.begin_node("")?; // Root node name is empty string
    // Use a common compatible for virtual machines. Can be platform specific.
    fdt.property_string("compatible", "riscv-virtio")?;
    // Standard for RV64: 2 cells (64 bits) for addresses and sizes
    fdt.property_u32("#address-cells", 0x2)?;
    fdt.property_u32("#size-cells", 0x2)?;

    // Chosen node: Kernel boot parameters
    let chosen_node = fdt.begin_node("chosen")?;
    // Kernel command line. 'console=hvc0' directs printk to OpenSBI console.
    // 'earlycon=sbi' enables very early messages via SBI before full console setup.
    // Add rootfs, init path etc. as needed, e.g. "root=/dev/vda rw"
    fdt.property_string("bootargs", "console=hvc earlycon=sbi verbose panic=-1")?; // FIXME:
    // (Optional but good practice) Specify path to console device if needed,
    // but hvc0 often doesn't require an explicit FDT node if handled purely via SBI.
    // fdt.property_string("stdout-path", "/soc/serial@10000000")?; // Example if UART existed
    fdt.end_node(chosen_node)?;

    // Memory node: Describes the main system RAM
    // Name follows convention: memory@<address>
    let memory_node = fdt.begin_node(&format!("memory@{:x}", GUEST_RAM_START))?;
    fdt.property_string("device_type", "memory")?;
    // Define the RAM region: start address and size. Uses address/size cells from root (2 each).
    fdt.property_array_u64("reg", &[GUEST_RAM_START, GUEST_RAM_SIZE])?;
    fdt.end_node(memory_node)?;

    // CPUs node: Container for CPU definitions
    let cpus_node = fdt.begin_node("cpus")?;
    // Within CPUs node, address-cells usually 1 (for hart ID)
    fdt.property_u32("#address-cells", 0x1)?;
    // Size-cells typically 0 for CPUs
    fdt.property_u32("#size-cells", 0x0)?;
    // Timer frequency: Essential for timekeeping. OpenSBI usually uses this.
    fdt.property_u32("timebase-frequency", GUEST_TIMEBASE_FREQ)?;

    // Define each CPU (hart)
    for i in 0..num_cpus.try_into().unwrap() {
        let hart_id = GUEST_HART_ID_BASE + i;
        // Name convention: cpu@<hartid>
        let cpu_node = fdt.begin_node(&format!("cpu@{:x}", hart_id))?;
        fdt.property_string("device_type", "cpu")?;
        // Use a generic RISC-V compatible string
        fdt.property_string("compatible", "riscv")?;
        // Hart ID for this CPU. Uses address-cells from 'cpus' node (1 cell).
        fdt.property_u32("reg", hart_id)?;
        // Mark the CPU as available
        fdt.property_string("status", "okay")?;
        // Specify the supported RISC-V ISA extensions
        fdt.property_string("riscv,isa-base", "rv64i")?;
        fdt.property_string_list(
            "riscv,isa-extensions",
            vec![
                "i".to_string(),
                "m".to_string(),
                "a".to_string(),
                "f".to_string(),
                "d".to_string(),
                "sstc".to_string(),
            ],
        )?;
        // Specify the MMU type (e.g., Sv39, Sv48)
        fdt.property_string("mmu-type", GUEST_MMU_TYPE)?;

        // Interrupt controller specific to this hart (handles timer, SW, external via S-mode trap)
        let intc_node = fdt.begin_node("interrupt-controller")?;
        fdt.property_u32("#interrupt-cells", 1)?; // Standard RISC-V: cell defines interrupt type
        fdt.property_null("interrupt-controller")?; // Marks this node as the controller
        fdt.property_string("compatible", "riscv,cpu-intc")?; // Standard compatible for per-hart intc
        fdt.end_node(intc_node)?;

        fdt.end_node(cpu_node)?;
    }
    fdt.end_node(cpus_node)?;

    // PLIC node.
    let plic_node = fdt.begin_node("plic")?;
    fdt.property_string("compatible", "riscv,plic0")?;
    fdt.property_u32("#address-cells", 0x1)?;
    fdt.property_u32("#size-cells", 0x0)?;
    fdt.property_array_u64("reg", &[plic_base.as_usize() as u64, plic_size(num_cpus)])?;
    fdt.end_node(plic_node)?;

    // Virtio-mmio devices.
    for (i, virtio_mmio) in virtio_mmios.iter().enumerate() {
        let virtio_mmio_node = fdt.begin_node(&format!("virtio-mmio@{}", i))?;
        fdt.property_string("compatible", "virtio,mmio")?;
        fdt.property_array_u64("reg", &[virtio_mmio.as_usize() as u64, 0x1000])?;
        fdt.end_node(virtio_mmio_node)?;
    }

    // Finish Root node
    fdt.end_node(root_node)?;

    // Finalize FDT blob
    fdt.finish()
}
