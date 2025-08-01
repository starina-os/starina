use core::mem::offset_of;

use starina::address::PAddr;
use starina::device_tree::DeviceTree;
use starina::info;
use starina::interrupt::Interrupt;
use starina::mmio::MmioRegion;
use starina::prelude::Box;
use starina::prelude::vec::Vec;
use starina_driver_sdk::DmaBufferPool;
use virtio::DeviceType;
use virtio::transports::VirtioTransport;
use virtio::transports::mmio::VirtioMmio;
use virtio::virtqueue::VirtQueue;
use virtio::virtqueue::VirtqDescBuffer;
use virtio::virtqueue::VirtqUsedChain;

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

fn probe(
    device_tree: &DeviceTree,
) -> Option<(Box<dyn VirtioTransport>, Vec<VirtQueue>, Interrupt)> {
    for (name, node) in &device_tree.devices {
        if !node.compatible.iter().any(|c| c == "virtio,mmio") {
            continue;
        }

        let paddr = PAddr::new(node.reg[0].addr as usize);
        let len = node.reg[0].size as usize;
        let folio = MmioRegion::pin(paddr, len).unwrap();
        let mut virtio = VirtioMmio::new(folio);
        let device_type = virtio.probe();

        if device_type == Some(DeviceType::Net) {
            info!("found virtio-net device: {}", name);
            let mut transport = Box::new(virtio) as Box<dyn VirtioTransport>;
            let virtqueues = transport.initialize(0, 2).unwrap();
            let interrupt =
                Interrupt::create(node.interrupts[0]).expect("failed to create interrupt");
            return Some((transport, virtqueues, interrupt));
        }
    }

    None
}

pub struct VirtioNet {
    mac_addr: [u8; 6],
    transport: Box<dyn VirtioTransport>,
    transmitq: VirtQueue,
    receiveq: VirtQueue,
    transmitq_buffers: DmaBufferPool,
    receiveq_buffers: DmaBufferPool,
    interrupt: Option<Interrupt>,
    receive: Option<Box<dyn for<'a> Fn(&'a [u8]) + Send + Sync>>,
}

impl VirtioNet {
    pub fn init_or_panic(device_tree: &DeviceTree) -> Self {
        let (mut transport, mut virtqueues, interrupt) = probe(device_tree).unwrap();
        assert!(transport.is_modern());

        let mut mac = [0; 6];
        for i in 0..6 {
            mac[i] = transport.read_device_config8((offset_of!(VirtioNetConfig, mac) + i) as u16)
        }

        let mut receiveq = virtqueues.remove(0 /* 1st queue */);
        let transmitq = virtqueues.remove(0 /* 2nd queue */);
        let mut receiveq_buffers = DmaBufferPool::new(DMA_BUF_SIZE, receiveq.num_descs() as usize);
        let transmitq_buffers = DmaBufferPool::new(DMA_BUF_SIZE, transmitq.num_descs() as usize);

        while let Some(i) = receiveq_buffers.allocate() {
            let chain = &[VirtqDescBuffer::WritableFromDevice {
                paddr: receiveq_buffers.paddr(i),
                len: DMA_BUF_SIZE,
            }];

            receiveq.enqueue(chain);
        }
        receiveq.notify(&mut *transport);

        Self {
            mac_addr: mac,
            transport,
            receiveq,
            transmitq,
            receiveq_buffers,
            transmitq_buffers,
            interrupt: Some(interrupt),
            receive: Some(Box::new(|_| {})),
        }
    }

    pub fn mac_addr(&self) -> &[u8; 6] {
        &self.mac_addr
    }

    pub fn take_interrupt(&mut self) -> Option<Interrupt> {
        self.interrupt.take()
    }

    pub fn set_receive_callback<F>(&mut self, callback: F)
    where
        F: for<'a> Fn(&'a [u8]) + Send + Sync + 'static,
    {
        self.receive = Some(Box::new(callback));
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
        let paddr = writer.finish();

        self.transmitq.enqueue(&[
            VirtqDescBuffer::ReadOnlyFromDevice {
                paddr,
                len: size_of::<VirtioNetModernHeader>(),
            },
            VirtqDescBuffer::ReadOnlyFromDevice {
                paddr: paddr.add(size_of::<VirtioNetModernHeader>()),
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
                    let VirtqDescBuffer::WritableFromDevice { paddr, len } = desc else {
                        panic!("unexpected desc");
                    };

                    let read_len = core::cmp::min(len, remaining);
                    remaining -= read_len;

                    let mut buf = self
                        .receiveq_buffers
                        .from_device(paddr)
                        .expect("invalid paddr");

                    let _header = buf.read::<VirtioNetModernHeader>();
                    let payload = buf.read_bytes(read_len).unwrap();

                    (self.receive.as_ref().unwrap())(payload);
                }
            }

            while let Some(VirtqUsedChain { descs, .. }) = self.transmitq.pop_used() {
                let VirtqDescBuffer::ReadOnlyFromDevice { paddr, .. } = descs[0] else {
                    panic!("unexpected desc");
                };
                let buffer_index = self
                    .transmitq_buffers
                    .paddr_to_id(paddr)
                    .expect("invalid paddr");
                self.transmitq_buffers.free(buffer_index);
            }

            while let Some(buffer_index) = self.receiveq_buffers.allocate() {
                let chain = &[VirtqDescBuffer::WritableFromDevice {
                    paddr: self.receiveq_buffers.paddr(buffer_index),
                    len: DMA_BUF_SIZE,
                }];

                self.receiveq.enqueue(chain);
            }

            self.receiveq.notify(&mut *self.transport);
        }
    }
}
