use starina::channel::Channel;
use starina::collections::HashMap;
use starina::handle::Handleable;
use starina::message::CallId;
use starina::message::Message;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::*;
use starina::vcpu::VCpu;
use starina_types::address::GPAddr;
use starina_types::vcpu::VCpuExit;
use starina_utils::static_assert;

use crate::Port;
use crate::fs::FileSystem;
use crate::guest_memory::GuestMemory;
use crate::interrupt::IrqTrigger;
use crate::mmio::Bus;
use crate::riscv::device_tree::build_fdt;
use crate::virtio::device::VIRTIO_MMIO_SIZE;
use crate::virtio::device::VirtioMmio;
use crate::virtio::virtio_fs::VirtioFs;
use crate::virtio::virtio_net::VirtioNet;

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

enum PortForward {
    TcpListen { listen_ch: Channel },
    TcpEstablished { data_ch: Channel },
}

enum State {
    Tcpip(Channel),
    PortForward(PortForward),
}

pub fn boot_linux(fs: FileSystem, ports: &[Port]) {
    let mut memory = GuestMemory::new(GUEST_RAM_ADDR, GUEST_RAM_SIZE).unwrap();

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
    )
    .expect("failed to build device tree");

    let (fdt_slice, fdt_gpaddr) = memory.allocate(fdt.len(), 4096).unwrap();
    fdt_slice[..fdt.len()].copy_from_slice(&fdt);

    // Prepare the guest memory.
    let entry = crate::linux_loader::load_riscv_image(&mut memory, LINUX_ELF).unwrap();

    let irq_trigger = IrqTrigger::new();

    let mut bus = Bus::new();
    let virtio_fs = VirtioFs::new(Box::new(fs));
    let virtio_net = VirtioNet::new();
    let virtio_mmio_fs = VirtioMmio::new(irq_trigger.clone(), VIRTIO_FS_IRQ, virtio_fs);
    let virtio_mmio_net = VirtioMmio::new(irq_trigger.clone(), VIRTIO_NET_IRQ, virtio_net);
    bus.add_device(VIRTIO_FS_ADDR, VIRTIO_MMIO_SIZE, virtio_mmio_fs);
    bus.add_device(VIRTIO_NET_ADDR, VIRTIO_MMIO_SIZE, virtio_mmio_net);

    let poll = Poll::new().unwrap();
    let tcpip_ch: Channel = todo!();
    poll.add(
        tcpip_ch.handle_id(),
        State::Tcpip(tcpip_ch),
        Readiness::READABLE | Readiness::CLOSED,
    )
    .unwrap();

    let mut remaining_ports = HashMap::with_capacity(ports.len());
    for (index, port) in ports.iter().enumerate() {
        let Port::Tcp { host, guest } = port;
        let open_call_id = CallId::from(index as u32);
        let uri = format!("tcp-listen:0.0.0.0:{}", host);
        tcpip_ch
            .send(Message::Open {
                call_id: open_call_id,
                uri: uri.as_bytes(),
            })
            .unwrap();
        remaining_ports.insert(open_call_id, port);
    }

    let mut poll2 = Poll::new().unwrap();
    while !remaining_ports.is_empty() {
        let (state, readiness) = poll.wait().unwrap();
        match &*state {
            State::Tcpip(ch) if readiness.contains(Readiness::READABLE) => {
                let mut m = ch.recv().unwrap();
                match m.parse() {
                    Some(Message::OpenReply { call_id, handle }) => {
                        poll2
                            .add(
                                handle.handle_id(),
                                State::PortForward(PortForward::TcpListen { listen_ch: handle }),
                                Readiness::READABLE,
                            )
                            .unwrap();
                        remaining_ports.remove(&call_id);
                    }
                    _ => {
                        panic!("unexpected message from tcpip: {:?}", m);
                    }
                }
            }
            _ => {
                panic!("unexpected state");
            }
        }
    }

    // Fill registers that Linux expects:
    //
    // > $a0 to contain the hartid of the current core.
    // > $a1 to contain the address of the devicetree in memory.
    // > https://www.kernel.org/doc/html/next/riscv/boot.html
    let a0 = 0; // hartid
    let a1 = fdt_gpaddr.as_usize(); // fdt address

    let mut vcpu = VCpu::new(memory.hvspace(), entry.as_usize(), a0, a1).unwrap();
    loop {
        let (state, readiness) = poll2.try_wait().unwrap();
        match &*state {
            State::PortForward(PortForward::TcpListen { listen_ch })
                if readiness.contains(Readiness::READABLE) =>
            {
                let mut m = listen_ch.recv().unwrap();
                match m.parse() {
                    Some(Message::Connect { handle }) => {
                        poll2.add(
                            handle.handle_id(),
                            State::PortForward(PortForward::TcpEstablished { data_ch: handle }),
                            Readiness::READABLE,
                        );
                    }
                    _ => {
                        panic!("listen-ch: unexpected message from tcpip: {:?}", m);
                    }
                }
            }
            State::PortForward(PortForward::TcpEstablished { data_ch })
                if readiness.contains(Readiness::READABLE) =>
            {
                let mut m = data_ch.recv().unwrap();
                match m.parse() {
                    Some(Message::StreamData { data }) => {
                        // port_forwarder.send()
                    }
                    _ => {
                        panic!("data-ch: unexpected message from tcpip: {:?}", m);
                    }
                }
            }
            _ => {
                panic!("poll2: unexpected state");
            }
        }

        vcpu.inject_irqs(irq_trigger.clear_all());
        let exit = vcpu.run().unwrap();
        match exit {
            VCpuExit::Reboot => {
                break;
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
