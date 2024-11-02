use core::mem::offset_of;

use starina_api::folio::MappedFolio;
use starina_api::interrupt::Interrupt;
use starina_api::prelude::*;
use starina_api::types::address::PAddr;
use starina_api::types::environ::Device;
use starina_api::types::interrupt::Irq;
use starina_driver_utils::DmaBufferPool;
use starina_virtio::transports::mmio::VirtioMmio;
use starina_virtio::transports::VirtioTransport;
use starina_virtio::virtqueue::VirtQueue;
use starina_virtio::virtqueue::VirtqDescBuffer;
use starina_virtio::virtqueue::VirtqUsedChain;
use starina_virtio::VIRTIO_DEVICE_TYPE_NET;

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

fn probe(devices: &[Device], device_type: u32) -> Option<(VirtioMmio, Irq)> {
    for device in devices {
        let base_paddr = PAddr::new(device.reg as usize);
        let mmio = MappedFolio::create_pinned(base_paddr, 0x200).unwrap();

        let mut transport = VirtioMmio::new(mmio);
        match transport.probe() {
            Some(ty) if ty == device_type => {
                #[cfg(target_arch = "aarch64")]
                let irq = Irq::from_raw(device.interrupts.as_ref().unwrap()[1] as usize + 32); // FIXME:
                #[cfg(not(target_arch = "aarch64"))]
                let irq = Irq::from_raw(device.interrupts.as_ref().unwrap()[0] as usize);

                return Some((transport, irq));
            }
            Some(ty) => {
                debug_warn!("unexpected device type: {}", ty);
            }
            None => {
                warn!("failed to probe the device");
            }
        }
    }

    None
}

pub struct VirtioNet {
    transport: Box<dyn VirtioTransport>,
    interrupt: Option<Interrupt>,
    transmitq: VirtQueue,
    transmitq_buffers: DmaBufferPool,
    receiveq: VirtQueue,
    receiveq_buffers: DmaBufferPool,
    mac: [u8; 6],
}

impl VirtioNet {
    pub fn new(devices: &[Device]) -> VirtioNet {
        let (mut transport, irq) = probe(devices, VIRTIO_DEVICE_TYPE_NET).unwrap();
        assert!(transport.is_modern());

        let mut mac = [0; 6];
        for i in 0..6 {
            mac[i] = transport.read_device_config8((offset_of!(VirtioNetConfig, mac) + i) as u16)
        }

        let interrupt = Interrupt::create(irq).unwrap();
        let mut transport = Box::new(transport) as Box<dyn VirtioTransport>;
        let mut virtqueues = transport.initialize(0, 2).unwrap();

        let mut receiveq = virtqueues.get_mut(0).unwrap().take().unwrap();
        let transmitq = virtqueues.get_mut(1).unwrap().take().unwrap();
        let mut receiveq_buffers = DmaBufferPool::new(DMA_BUF_SIZE, receiveq.num_descs() as usize);
        let transmitq_buffers = DmaBufferPool::new(DMA_BUF_SIZE, transmitq.num_descs() as usize);

        // Fill the receive queue with buffers.
        while let Some(buffer_index) = receiveq_buffers.allocate() {
            let chain = &[VirtqDescBuffer::WritableFromDevice {
                paddr: receiveq_buffers.paddr(buffer_index),
                len: DMA_BUF_SIZE,
            }];

            receiveq.enqueue(chain);
        }
        receiveq.notify(&mut *transport);

        VirtioNet {
            transport,
            interrupt: Some(interrupt),
            transmitq,
            transmitq_buffers,
            receiveq,
            receiveq_buffers,
            mac,
        }
    }

    pub fn hwaddr(&self) -> &[u8; 6] {
        &self.mac
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
        let paddr = self.transmitq_buffers.paddr(buffer_index);

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
                paddr,
                len: header_len,
            },
            VirtqDescBuffer::ReadOnlyFromDevice {
                paddr: paddr.add(header_len),
                len: payload.len(),
            },
        ];

        self.transmitq.enqueue(chain);
        self.transmitq.notify(&mut *self.transport);
    }

    pub fn handle_interrupt<F>(&mut self, mut receive: F)
    where
        F: FnMut(&[u8]),
    {
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

                    let buffer_index = self
                        .receiveq_buffers
                        .paddr_to_id(paddr)
                        .expect("invalid paddr");
                    let vaddr = self.receiveq_buffers.vaddr(buffer_index);
                    let header_len = size_of::<VirtioNetModernHeader>();
                    let payload = unsafe {
                        core::slice::from_raw_parts(vaddr.as_ptr::<u8>().add(header_len), read_len)
                    };

                    receive(payload);
                    self.receiveq_buffers.free(buffer_index);
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
