use starina::address::GPAddr;
use starina::prelude::*;
use vm_fdt::FdtWriter;

use crate::guest_net::GuestNet;
use crate::virtio::device::VIRTIO_MMIO_SIZE;

const TIMEBASE_FREQ: u32 = 10000000;

pub fn build_fdt(
    num_cpus: u32,
    guest_ram_start: GPAddr,
    guest_ram_size: u64,
    plic_base: GPAddr,
    plic_mmio_size: usize,
    virtio_mmios: &[(GPAddr, u8 /* irq */)],
    guest_net: &GuestNet,
) -> Result<Vec<u8>, vm_fdt::Error> {
    let mut fdt = FdtWriter::new()?;

    let root_node = fdt.begin_node("")?;
    fdt.property_string("compatible", "riscv-virtio")?;
    fdt.property_u32("#address-cells", 0x2)?;
    fdt.property_u32("#size-cells", 0x2)?;

    let chosen_node = fdt.begin_node("chosen")?;
    let ip_param = guest_net.build_linux_ip_param();
    let bootargs = format!("console=hvc earlycon=sbi quiet panic=-1 {}", ip_param);
    fdt.property_string("bootargs", &bootargs)?;
    fdt.end_node(chosen_node)?;

    let memory_node = fdt.begin_node(&format!("memory@{:x}", guest_ram_start.as_usize()))?;
    fdt.property_string("device_type", "memory")?;
    fdt.property_array_u64("reg", &[guest_ram_start.as_usize() as u64, guest_ram_size])?;
    fdt.end_node(memory_node)?;

    const PLIC_PHANDLE: u32 = 1;
    let mut next_phandle = 2;
    let mut interrupts_extended = vec![];

    // Define CPUs.
    let cpus_node = fdt.begin_node("cpus")?;
    fdt.property_u32("#address-cells", 0x1)?;
    fdt.property_u32("#size-cells", 0x0)?;
    fdt.property_u32("timebase-frequency", TIMEBASE_FREQ)?;
    for hart_id in 0..num_cpus {
        let cpu_node = fdt.begin_node(&format!("cpu@{:x}", hart_id))?;
        fdt.property_string("device_type", "cpu")?;
        fdt.property_string("compatible", "riscv")?;
        fdt.property_u32("reg", hart_id)?;
        fdt.property_string("status", "okay")?;
        fdt.property_string("mmu-type", "riscv,sv48")?;
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

        let phandle = next_phandle;
        next_phandle += 1;

        let intc_node = fdt.begin_node("interrupt-controller")?;
        fdt.property_u32("#interrupt-cells", 1)?;
        fdt.property_u32("#address-cells", 0)?;
        fdt.property_string("compatible", "riscv,cpu-intc")?;
        fdt.property_null("interrupt-controller")?;
        fdt.property_phandle(phandle)?;
        fdt.end_node(intc_node)?;

        interrupts_extended.push(phandle);
        interrupts_extended.push(0x9); // context 9

        fdt.end_node(cpu_node)?;
    }
    fdt.end_node(cpus_node)?;

    // From now on, define peripherals.
    let soc_node = fdt.begin_node("soc")?;
    fdt.property_u32("#address-cells", 0x2)?;
    fdt.property_u32("#size-cells", 0x2)?;
    fdt.property_string("compatible", "simple-bus")?;
    fdt.property_null("ranges")?;

    // PLIC node.
    let plic_node = fdt.begin_node(&format!("plic@{:x}", plic_base.as_usize()))?;
    fdt.property_string("compatible", "riscv,plic0")?;
    fdt.property_phandle(PLIC_PHANDLE)?;
    fdt.property_u32("#address-cells", 0)?;
    fdt.property_u32("#interrupt-cells", 1)?;
    fdt.property_u32("riscv,ndev", 3)?;
    fdt.property_null("interrupt-controller")?;
    fdt.property_array_u64("reg", &[0x0a00_0000, plic_mmio_size as u64])?;
    fdt.property_array_u32("interrupts-extended", &interrupts_extended)?;
    fdt.end_node(plic_node)?;

    // Virtio-mmio devices.
    for (gpaddr, irq) in virtio_mmios.iter() {
        let virtio_mmio_node = fdt.begin_node(&format!("virtio-mmio@{}", gpaddr.as_usize()))?;
        fdt.property_string("compatible", "virtio,mmio")?;
        fdt.property_array_u64(
            "reg",
            &[
                gpaddr.as_usize().try_into().unwrap(),
                VIRTIO_MMIO_SIZE as u64,
            ],
        )?;

        fdt.property_array_u32("interrupts", &[*irq as u32])?;
        fdt.property_u32("interrupt-parent", PLIC_PHANDLE)?;
        fdt.end_node(virtio_mmio_node)?;
    }

    fdt.end_node(soc_node)?;
    fdt.end_node(root_node)?;
    fdt.finish()
}
