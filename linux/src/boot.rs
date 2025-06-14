use core::time::Duration;

use starina::channel::Channel;
use starina::handle::Handleable;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::*;
use starina::sync::Arc;
use starina::sync::Mutex;
use starina::timer::Timer;
use starina::vcpu::VCpu;
use starina_types::address::GPAddr;
use starina_types::vcpu::VCpuExit;
use starina_utils::static_assert;

use crate::port_forward::Port;
use crate::fs::FileSystem;
use crate::guest_memory::GuestMemory;
use crate::guest_net::GuestNet;
use crate::interrupt::IrqTrigger;
use crate::mmio::Bus;
use crate::port_forward;
use crate::riscv::device_tree::build_fdt;
use crate::virtio::device::VIRTIO_MMIO_SIZE;
use crate::virtio::device::VirtioMmio;
use crate::virtio::virtio_fs::VirtioFs;
use crate::virtio::virtio_net::VirtioNet;
use crate::virtio::virtqueue::DescChainReader;

const fn plic_mmio_size(num_cpus: u32) -> usize {
    0x200000 + (num_cpus as usize * 0x1000)
}

const LINUX_ELF: &[u8] = include_bytes!("../kernel/arch/riscv/boot/Image");
const NUM_CPUS: u32 = 1;
const GUEST_RAM_SIZE: usize = 64 * 1024 * 1024; // 64MB
const PLIC_BASE_ADDR: GPAddr = GPAddr::new(0x0a00_0000);
const PLIC_MMIO_SIZE: usize = plic_mmio_size(NUM_CPUS);
static_assert!(PLIC_BASE_ADDR.as_usize() + PLIC_MMIO_SIZE <= VIRTIO_FS_ADDR.as_usize());
const VIRTIO_FS_ADDR: GPAddr = GPAddr::new(0x0b00_0000);
const VIRTIO_NET_ADDR: GPAddr = GPAddr::new(0x0b00_1000);
const VIRTIO_FS_IRQ: u8 = 1;
const VIRTIO_NET_IRQ: u8 = 2;
const GUEST_RAM_ADDR: GPAddr = GPAddr::new(0x8000_0000);

pub fn boot_linux(fs: FileSystem, ports: &[Port], tcpip_ch: Channel) {
    let mut memory = GuestMemory::new(GUEST_RAM_ADDR, GUEST_RAM_SIZE).unwrap();

    // Network configuration (moved from device_tree.rs)
    let guest_ip = crate::guest_net::Ipv4Addr::new(10, 255, 0, 100);
    let host_ip = crate::guest_net::Ipv4Addr::new(10, 255, 0, 1); // gateway IP
    let gw_ip = crate::guest_net::Ipv4Addr::new(10, 255, 0, 1);
    let netmask = crate::guest_net::Ipv4Addr::new(255, 255, 255, 0);
    let dns_servers = [
        crate::guest_net::Ipv4Addr::new(8, 8, 8, 8),
        crate::guest_net::Ipv4Addr::new(8, 8, 4, 4),
    ];
    let guest_mac = crate::guest_net::MacAddr::new([0x00, 0x00, 0x00, 0x00, 0x00, 0x01]);
    let host_mac = crate::guest_net::MacAddr::new([0x00, 0x00, 0x00, 0x00, 0x00, 0x02]);

    let guest_net = GuestNet::new(
        host_ip,
        guest_ip,
        guest_mac,
        host_mac,
        gw_ip,
        netmask,
        dns_servers,
    );

    let fdt = build_fdt(
        NUM_CPUS,
        GUEST_RAM_ADDR,
        GUEST_RAM_SIZE as u64,
        PLIC_BASE_ADDR,
        PLIC_MMIO_SIZE,
        &[
            (VIRTIO_FS_ADDR, VIRTIO_FS_IRQ),
            (VIRTIO_NET_ADDR, VIRTIO_NET_IRQ),
        ],
        &guest_net,
    )
    .expect("failed to build device tree");

    let guest_net = Arc::new(Mutex::new(guest_net));
    let packet_receiver: Box<dyn for<'a> Fn(DescChainReader<'a>)> = {
        let guest_net = guest_net.clone();
        Box::new(move |pkt| {
            guest_net.lock().recv_from_guest(pkt).unwrap();
        })
    };

    let (fdt_slice, fdt_gpaddr) = memory.allocate(fdt.len(), 4096).unwrap();
    fdt_slice[..fdt.len()].copy_from_slice(&fdt);

    // Prepare the guest memory.
    let entry = crate::linux_loader::load_riscv_image(&mut memory, LINUX_ELF).unwrap();

    let irq_trigger = IrqTrigger::new();

    let mut bus = Bus::new();
    let virtio_fs = VirtioFs::new(Box::new(fs));
    let virtio_net = VirtioNet::new(guest_mac, packet_receiver);
    let virtio_mmio_fs = Arc::new(VirtioMmio::new(
        irq_trigger.clone(),
        VIRTIO_FS_IRQ,
        virtio_fs,
    ));
    let virtio_mmio_net = Arc::new(VirtioMmio::new(
        irq_trigger.clone(),
        VIRTIO_NET_IRQ,
        virtio_net,
    ));
    bus.add_device(VIRTIO_FS_ADDR, VIRTIO_MMIO_SIZE, virtio_mmio_fs);
    bus.add_device(VIRTIO_NET_ADDR, VIRTIO_MMIO_SIZE, virtio_mmio_net.clone());

    let mut port_forwarder =
        port_forward::Builder::new(tcpip_ch, guest_net.clone(), virtio_mmio_net.clone(), ports)
            .build();

    let timer = Timer::new().unwrap();
    let timer_poll = Poll::new().unwrap();
    timer_poll
        .add(timer.handle_id(), (), Readiness::READABLE)
        .unwrap();

    // Fill registers that Linux expects:
    //
    // > $a0 to contain the hartid of the current core.
    // > $a1 to contain the address of the devicetree in memory.
    // > https://www.kernel.org/doc/html/next/riscv/boot.html
    let a0 = 0; // hartid
    let a1 = fdt_gpaddr.as_usize(); // fdt address

    let mut vcpu = VCpu::new(memory.hvspace(), entry.as_usize(), a0, a1).unwrap();

    loop {
        port_forwarder.poll(&mut memory);

        let irqs = irq_trigger.clear_all();
        vcpu.inject_irqs(irqs);

        let exit = vcpu.run().unwrap();
        match exit {
            VCpuExit::Reboot => break,
            VCpuExit::Idle => {
                // FIXME:
                timer.set_timeout(Duration::from_millis(1)).unwrap();
                timer_poll.wait().unwrap();
            }
            VCpuExit::LoadPageFault { gpaddr, data } => {
                bus.read(&mut memory, gpaddr, data).unwrap();
            }
            VCpuExit::StorePageFault { gpaddr, data } => {
                bus.write(&mut memory, gpaddr, data).unwrap();
            }
        }
    }
}
