use core::mem::offset_of;

use starina::address::DAddr;
use starina::folio::MmioFolio;
use starina::info;
use starina::interrupt::Interrupt;
use starina::iobus::IoBus;
use starina::prelude::Box;
use starina::prelude::vec::Vec;
use starina_driver_sdk::DmaBufferPool;
use virtio::DeviceType;
use virtio::transports::VirtioTransport;
use virtio::transports::mmio::VirtioMmio;
use virtio::virtqueue::VirtQueue;
use virtio::virtqueue::VirtqDescBuffer;
use virtio::virtqueue::VirtqUsedChain;

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
        let folio = MmioFolio::create_pinned(iobus, daddr, len).unwrap();
        let mut virtio = VirtioMmio::new(folio);
        let device_type = virtio.probe();

        if device_type == Some(DeviceType::Net) {
            info!("found virtio-net device: {}", name);
            let mut transport = Box::new(virtio) as Box<dyn VirtioTransport>;
            let virtqueues = transport.initialize(iobus, 0, 2).unwrap();
            let iobus = env.iobus.remove(&node.bus).unwrap();
            let interrupt =
                Interrupt::create(node.interrupts[0]).expect("failed to create interrupt");
            return Some((iobus, transport, virtqueues, interrupt));
        }
    }

    None
}

pub struct VirtioNet {
    mac_addr: [u8; 6],
    _iobus: IoBus,
    transport: Box<dyn VirtioTransport>,
    transmitq: VirtQueue,
    receiveq: VirtQueue,
    transmitq_buffers: DmaBufferPool,
    receiveq_buffers: DmaBufferPool,
    interrupt: Option<Interrupt>,
    receive: Option<Box<dyn FnMut(&[u8]) + Send + Sync>>,
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

        while let Some(i) = receiveq_buffers.allocate() {
            let chain = &[VirtqDescBuffer::WritableFromDevice {
                daddr: receiveq_buffers.daddr(i),
                len: DMA_BUF_SIZE,
            }];

            receiveq.enqueue(chain);
        }
        receiveq.notify(&mut *transport);

        Self {
            _iobus: iobus,
            mac_addr: mac,
            transport,
            receiveq,
            transmitq,
            receiveq_buffers,
            transmitq_buffers,
            interrupt: Some(interrupt),
            receive: None,
        }
    }

    pub fn update_receive(&mut self, f: Box<dyn FnMut(&[u8]) + Send + Sync>) {
        self.receive = Some(f);
    }

    pub fn mac_addr(&self) -> &[u8; 6] {
        &self.mac_addr
    }

    pub fn take_interrupt(&mut self) -> Option<Interrupt> {
        self.interrupt.take()
    }

    pub fn transmit(&mut self, payload: &[u8]) {
        let mut writer = self.transmitq_buffers.to_device().unwrap();
        writer
            .write(VirtioNetModernHeader {
                flags: 0,
                hdr_len: 0,
                gso_type: 0,
                gso_size: 0,
                checksum_start: 0,
                checksum_offset: 0,
                // num_buffer: 0,
            })
            .unwrap();
        writer.write_bytes(payload).unwrap();
        let daddr = writer.finish();

        self.transmitq.enqueue(&[
            VirtqDescBuffer::ReadOnlyFromDevice {
                daddr,
                len: size_of::<VirtioNetModernHeader>(),
            },
            VirtqDescBuffer::ReadOnlyFromDevice {
                daddr: daddr.add(size_of::<VirtioNetModernHeader>()),
                len: payload.len(),
            },
        ]);
        self.transmitq.notify(&mut *self.transport);
    }

    pub fn handle_interrupt(&mut self) {
        loop {
            let status = self.transport.read_isr_status();
            self.transport.ack_interrupt(status);

            if !status.queue_intr() {
                break;
            }

            while let Some(VirtqUsedChain { descs, total_len }) = self.receiveq.pop_used() {
                debug_assert!(descs.len() == 1);
                let mut remaining = total_len;
                for desc in descs {
                    let VirtqDescBuffer::WritableFromDevice { daddr, len } = desc else {
                        panic!("unexpected desc");
                    };

                    let read_len = core::cmp::min(len, remaining);
                    remaining -= read_len;

                    let mut buf = self
                        .receiveq_buffers
                        .from_device(daddr)
                        .expect("invalid daddr");

                    let _header = buf.read::<VirtioNetModernHeader>();
                    let payload = buf.read_bytes(read_len).unwrap();
                    if let Some(receive) = self.receive.as_mut() {
                        receive(payload);
                    }
                }
            }

            while let Some(VirtqUsedChain { descs, .. }) = self.transmitq.pop_used() {
                let VirtqDescBuffer::ReadOnlyFromDevice { daddr, .. } = descs[0] else {
                    panic!("unexpected desc");
                };
                let buffer_index = self
                    .transmitq_buffers
                    .daddr_to_id(daddr)
                    .expect("invalid daddr");
                self.transmitq_buffers.free(buffer_index);
            }

            while let Some(buffer_index) = self.receiveq_buffers.allocate() {
                let chain = &[VirtqDescBuffer::WritableFromDevice {
                    daddr: self.receiveq_buffers.daddr(buffer_index),
                    len: DMA_BUF_SIZE,
                }];

                self.receiveq.enqueue(chain);
            }

            self.receiveq.notify(&mut *self.transport);
        }
    }
}
