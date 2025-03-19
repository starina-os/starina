use core::mem::offset_of;

use starina::address::DAddr;
use starina::device_tree::InterruptDesc;
use starina::folio::MmioFolio;
use starina::info;
use starina::interrupt::Interrupt;
use starina::iobus::IoBus;
use starina::prelude::Box;
use starina::prelude::vec::Vec;
use starina::trace;
use starina_driver_sdk::DmaBufferPool;
use virtio::DeviceType;
use virtio::transports::VirtioTransport;
use virtio::transports::mmio::VirtioMmio;
use virtio::virtqueue::VirtQueue;
use virtio::virtqueue::VirtqDescBuffer;

use crate::autogen::Env;

const DMA_BUF_SIZE: usize = 4096;

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct VirtioNetModernHeader {
    flags: u8,
    gso_type: u8,
    hdr_len: u16,
    gso_size: u16,
    checksum_start: u16,
    checksum_offset: u16,
    // num_buffer: u16,
}

#[repr(C, packed)]
struct VirtioNetConfig {
    mac: [u8; 6],
    status: u16,
    max_virtqueue_pairs: u16,
    mtu: u16,
    speed: u32,
    duplex: u8,
    rss_max_key_size: u8,
    rss_max_indirection_table_length: u16,
    supported_hash_types: u32,
}

fn probe(mut env: Env) -> Option<(IoBus, Box<dyn VirtioTransport>, Vec<VirtQueue>, Interrupt)> {
    for (name, node) in env.device_tree.devices {
        if !node.compatible.iter().any(|c| c == "virtio,mmio") {
            continue;
        }

        let iobus = env.iobus.get(&node.bus).expect("missing iobus");
        let daddr = DAddr::new(node.reg[0].addr as usize);
        let len = node.reg[0].size as usize;
        let folio = MmioFolio::create_pinned(&iobus, daddr, len).unwrap();
        let mut virtio = VirtioMmio::new(folio);
        let device_type = virtio.probe();

        if device_type == Some(DeviceType::Net) {
            info!("found virtio-net device: {}", name);
            let mut transport = Box::new(virtio) as Box<dyn VirtioTransport>;
            let virtqueues = transport.initialize(&iobus, 0, 2).unwrap();
            let iobus = env.iobus.remove(&node.bus).unwrap();
            let interrupt = match node.interrupts[0] {
                InterruptDesc::Static(irq) => {
                    Interrupt::create(irq).expect("failed to create interrupt")
                }
                _ => panic!("invalid interrupt descriptor"),
            };
            return Some((iobus, transport, virtqueues, interrupt));
        }
    }

    None
}

pub struct VirtioNet {
    mac: [u8; 6],
    _iobus: IoBus,
    transport: Box<dyn VirtioTransport>,
    transmitq: VirtQueue,
    receiveq: VirtQueue,
    transmitq_buffers: DmaBufferPool,
    receiveq_buffers: DmaBufferPool,
    interrupt: Option<Interrupt>,
}

impl VirtioNet {
    pub fn init_or_panic(env: Env) -> Self {
        let (iobus, mut transport, mut virtqueues, interrupt) = probe(env).unwrap();
        assert!(transport.is_modern());

        let mut mac = [0; 6];
        for i in 0..6 {
            mac[i] = transport.read_device_config8((offset_of!(VirtioNetConfig, mac) + i) as u16)
        }

        let mut receiveq = virtqueues.remove(0 /* 1st queue */);
        let transmitq = virtqueues.remove(0 /* 2nd queue */);
        let mut receiveq_buffers =
            DmaBufferPool::new(&iobus, DMA_BUF_SIZE, receiveq.num_descs() as usize);
        let transmitq_buffers =
            DmaBufferPool::new(&iobus, DMA_BUF_SIZE, transmitq.num_descs() as usize);

        while let Some(buffer_index) = receiveq_buffers.allocate() {
            let chain = &[VirtqDescBuffer::WritableFromDevice {
                daddr: receiveq_buffers.daddr(buffer_index),
                len: DMA_BUF_SIZE,
            }];

            receiveq.enqueue(chain);
        }
        receiveq.notify(&mut *transport);

        Self {
            _iobus: iobus,
            mac,
            transport,
            receiveq,
            transmitq,
            receiveq_buffers,
            transmitq_buffers,
            interrupt: Some(interrupt),
        }
    }

    pub fn take_interrupt(&mut self) -> Option<Interrupt> {
        self.interrupt.take()
    }

    pub fn transmit(&mut self, payload: &[u8]) {
        let buffer_index = self
            .transmitq_buffers
            .allocate()
            .expect("no free tx buffers");
        let vaddr = self.transmitq_buffers.vaddr(buffer_index);
        let daddr = self.transmitq_buffers.daddr(buffer_index);

        unsafe {
            vaddr
                .as_mut_ptr::<VirtioNetModernHeader>()
                .write(VirtioNetModernHeader {
                    flags: 0,
                    hdr_len: 0,
                    gso_type: 0,
                    gso_size: 0,
                    checksum_start: 0,
                    checksum_offset: 0,
                    // num_buffer: 0,
                });
        }

        let header_len = size_of::<VirtioNetModernHeader>();
        unsafe {
            let buf = core::slice::from_raw_parts_mut(
                vaddr.add(header_len).as_mut_ptr(),
                DMA_BUF_SIZE - header_len,
            );
            buf[..payload.len()].copy_from_slice(payload);
        }

        let chain = &[
            VirtqDescBuffer::ReadOnlyFromDevice {
                daddr,
                len: header_len,
            },
            VirtqDescBuffer::ReadOnlyFromDevice {
                daddr: daddr.add(header_len),
                len: payload.len(),
            },
        ];

        self.transmitq.enqueue(chain);
        self.transmitq.notify(&mut *self.transport);
    }
}
