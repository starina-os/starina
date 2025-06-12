use core::time::Duration;

use starina::channel::Channel;
use starina::channel::ChannelReceiver;
use starina::collections::HashMap;
use starina::error::ErrorCode;
use starina::handle::Handleable;
use starina::message::CallId;
use starina::message::Message;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::*;
use starina::sync::Arc;
use starina::timer::Timer;
use starina::vcpu::VCpu;
use starina_types::address::GPAddr;
use starina_types::vcpu::VCpuExit;
use starina_utils::static_assert;

use crate::Port;
use crate::fs::FileSystem;
use crate::guest_memory::GuestMemory;
use crate::guest_net::ConnKey;
use crate::guest_net::GuestNet;
use crate::guest_net::IpProto;
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
    Tcp {
        listen_ch: Channel,
        guest_port: u16,
    },
    TcpEstablished {
        receiver: ChannelReceiver,
        conn_key: ConnKey,
    },
}

enum State {
    Tcpip(ChannelReceiver),
    PortForward(PortForward),
}

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

    let (fdt_slice, fdt_gpaddr) = memory.allocate(fdt.len(), 4096).unwrap();
    fdt_slice[..fdt.len()].copy_from_slice(&fdt);

    // Prepare the guest memory.
    let entry = crate::linux_loader::load_riscv_image(&mut memory, LINUX_ELF).unwrap();

    let irq_trigger = IrqTrigger::new();

    let mut bus = Bus::new();
    let virtio_fs = VirtioFs::new(Box::new(fs));
    let virtio_net = VirtioNet::new(guest_net, guest_mac);
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

    let poll = Poll::new().unwrap();
    let (tcpip_tx, tcpip_rx) = tcpip_ch.split();
    poll.add(
        tcpip_rx.handle().id(),
        State::Tcpip(tcpip_rx),
        Readiness::READABLE | Readiness::CLOSED,
    )
    .unwrap();

    let mut remaining_ports = HashMap::with_capacity(ports.len());
    for (index, port) in ports.iter().enumerate() {
        let Port::Tcp { host, .. } = port;
        let open_call_id = CallId::from(index as u32);
        let uri = format!("tcp-listen:0.0.0.0:{}", host);
        tcpip_tx
            .send(Message::Open {
                call_id: open_call_id,
                uri: uri.as_bytes(),
            })
            .unwrap();
        remaining_ports.insert(open_call_id, port);
    }

    info!("waiting for tcpip to open ports...");

    let poll2 = Poll::new().unwrap();
    while !remaining_ports.is_empty() {
        let (state, readiness) = poll.wait().unwrap();
        match &*state {
            State::Tcpip(ch) if readiness.contains(Readiness::READABLE) => {
                let mut m = ch.recv().unwrap();
                match m.parse() {
                    Some(Message::OpenReply { call_id, handle }) => {
                        let port = remaining_ports.remove(&call_id).unwrap();
                        let Port::Tcp {
                            guest: guest_port, ..
                        } = *port;
                        poll2
                            .add(
                                handle.handle_id(),
                                State::PortForward(PortForward::Tcp {
                                    listen_ch: handle,
                                    guest_port,
                                }),
                                Readiness::READABLE,
                            )
                            .unwrap();
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

    info!("all ports are open");

    let timer = Timer::new().unwrap();
    let poll3 = Poll::new().unwrap();
    poll3
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
        'poll_loop: loop {
            match poll2.try_wait() {
                Ok((state, readiness)) => {
                    match &*state {
                        State::PortForward(PortForward::Tcp {
                            listen_ch,
                            guest_port,
                        }) if readiness.contains(Readiness::READABLE) => {
                            let mut m = listen_ch.recv().unwrap();
                            match m.parse() {
                                Some(Message::Connect { handle }) => {
                                    info!("connect: {:?}", handle);

                                    // Split the data channel into sender and receiver
                                    let (sender, receiver) = handle.split();

                                    // Create forwarder closure that sends StreamData when receiving TCP data from guest
                                    let forwarder =
                                        Box::new(move |_conn_key: &ConnKey, data: &[u8]| {
                                            sender.send(Message::StreamData { data }).unwrap();
                                        });

                                    // Connect to guest with the new API that automatically assigns remote port
                                    let conn_key = virtio_mmio_net.use_vq(0, |device, vq| {
                                        device.connect_to_guest(
                                            &mut memory,
                                            vq,
                                            *guest_port,
                                            IpProto::Tcp,
                                            forwarder,
                                        )
                                    });

                                    poll2
                                        .add(
                                            receiver.handle().id(),
                                            State::PortForward(PortForward::TcpEstablished {
                                                receiver,
                                                conn_key,
                                            }),
                                            Readiness::READABLE,
                                        )
                                        .unwrap();
                                }
                                _ => {
                                    panic!("listen-ch: unexpected message from tcpip: {:?}", m);
                                }
                            }
                        }
                        State::PortForward(PortForward::TcpEstablished { receiver, conn_key })
                            if readiness.contains(Readiness::READABLE) =>
                        {
                            let mut m = receiver.recv().unwrap();
                            match m.parse() {
                                Some(Message::StreamData { data }) => {
                                    virtio_mmio_net.use_vq(0, |device, vq| {
                                        device.send_to_guest(&mut memory, vq, conn_key, data);
                                    });
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
                }
                Err(ErrorCode::WouldBlock) => {
                    break 'poll_loop;
                }
                Err(e) => {
                    panic!("poll2: {:?}", e);
                }
            }
        }

        virtio_mmio_net.use_vq(0 /* tx */, |device, vq| {
            device.flush_to_guest(&mut memory, vq);
        });

        let irqs = irq_trigger.clear_all();
        vcpu.inject_irqs(irqs);
        let exit = vcpu.run().unwrap();
        match exit {
            VCpuExit::Reboot => {
                break;
            }
            VCpuExit::Idle => {
                // FIXME:
                timer.set_timeout(Duration::from_millis(1)).unwrap();
                poll3.wait().unwrap();
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
