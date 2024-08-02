#![no_std]
#![no_main]

use ftl_api::channel::Channel;
use ftl_api::folio::MmioFolio;
use ftl_api::handle::OwnedHandle;
use ftl_api::interrupt::Interrupt;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_api::types::address::PAddr;
use ftl_api::types::address::VAddr;
use ftl_api::types::environ::Device;
use ftl_api::types::idl::BytesField;
use ftl_api::types::interrupt::Irq;
use ftl_api::types::message::MessageBuffer;
use ftl_api_autogen::apps::virtio_net::Environ;
use ftl_api_autogen::apps::virtio_net::Message;
use ftl_api_autogen::protocols::ethernet_device;
use ftl_virtio::transports::mmio::VirtioMmio;
use ftl_virtio::transports::VirtioTransport;
use ftl_virtio::virtqueue::align_up;
use ftl_virtio::virtqueue::VirtqDescBuffer;
use ftl_virtio::virtqueue::VirtqUsedChain;
use ftl_virtio::VIRTIO_DEVICE_TYPE_NET;

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

#[derive(Copy, Clone)]
pub struct BufferIndex(usize);

pub struct BufferPool {
    folio: MmioFolio,
    free_indices: Vec<BufferIndex>,
    buffer_size: usize,
    num_buffers: usize,
}

impl BufferPool {
    pub fn new(buffer_size: usize, num_buffers: usize) -> BufferPool {
        let folio = MmioFolio::create(align_up(buffer_size * num_buffers, 4096)).unwrap();
        let mut free_indices = Vec::new();
        for i in 0..num_buffers {
            free_indices.push(BufferIndex(i));
        }

        BufferPool {
            folio,
            free_indices,
            buffer_size,
            num_buffers,
        }
    }

    pub fn pop_free(&mut self) -> Option<BufferIndex> {
        self.free_indices.pop()
    }

    pub fn push_free(&mut self, index: BufferIndex) {
        self.free_indices.push(index);
    }

    pub fn paddr_to_index(&self, paddr: PAddr) -> Option<BufferIndex> {
        debug_assert!(
            paddr.as_usize() % self.buffer_size == 0,
            "paddr is not aligned"
        );

        // TODO: paddr may not be in the same folio
        let offset = paddr.as_usize() - self.folio.paddr().as_usize();
        let index = offset / self.buffer_size;
        if index < self.num_buffers {
            Some(BufferIndex(index))
        } else {
            None
        }
    }

    pub fn vaddr(&self, index: BufferIndex) -> VAddr {
        debug_assert!(index.0 < self.num_buffers);
        self.folio.vaddr().add(index.0 * self.buffer_size)
    }

    pub fn paddr(&self, index: BufferIndex) -> PAddr {
        debug_assert!(index.0 < self.num_buffers);
        self.folio.paddr().add(index.0 * self.buffer_size)
    }
}

fn probe(devices: &[Device], device_type: u32) -> Option<(VirtioMmio, Irq)> {
    for device in devices {
        let base_paddr = PAddr::new(device.reg as usize).unwrap();
        let mmio = MmioFolio::create_pinned(base_paddr, 0x1000).unwrap();

        let mut transport = VirtioMmio::new(mmio);
        match transport.probe() {
            Some(ty) if ty == device_type => {
                info!("console: IRQs: {:?}", device.interrupts);
                let irq = Irq::from_raw(device.interrupts.as_ref().unwrap()[1] as usize + 32); // FIXME:
                return Some((transport, irq));
            }
            Some(ty) => {
                warn!("unexpected device type: {}", ty);
            }
            None => {
                warn!("failed to probe the device");
            }
        }
    }

    None
}

enum Context {
    Autopilot,
    Interrupt,
    Tcpip,
}

#[ftl_api::main]
pub fn main(mut env: Environ) {
    info!("starting");
    let mut buffer = MessageBuffer::new();

    let (mut transport, irq) = probe(&env.depends.virtio, VIRTIO_DEVICE_TYPE_NET).unwrap();
    assert!(transport.is_modern());

    let interrupt = Interrupt::create(irq).unwrap();
    let mut transport = Box::new(transport) as Box<dyn VirtioTransport>;
    let mut virtqueues = transport.initialize(0, 2).unwrap();

    let mut transmitq = virtqueues.get_mut(1).unwrap().take().unwrap();
    let mut receiveq = virtqueues.get_mut(0).unwrap().take().unwrap();
    let dma_buf_len = 4096;
    let mut receiveq_buffers = BufferPool::new(dma_buf_len, receiveq.num_descs() as usize);
    let mut transmitq_buffers = BufferPool::new(dma_buf_len, transmitq.num_descs() as usize);

    // Fill the receive queue with buffers.
    while let Some(buffer_index) = receiveq_buffers.pop_free() {
        let chain = &[VirtqDescBuffer::WritableFromDevice {
            paddr: receiveq_buffers.paddr(buffer_index),
            len: dma_buf_len,
        }];

        receiveq.enqueue(chain);
    }
    receiveq.notify(&mut *transport);

    let mut mainloop = Mainloop::<Context, Message>::new().unwrap();
    mainloop
        .add_channel(env.autopilot_ch.take().unwrap(), Context::Autopilot)
        .unwrap();
    mainloop
        .add_interrupt(interrupt, Context::Interrupt)
        .unwrap();

    let mut tcpip_ch = None;
    loop {
        match mainloop.next(&mut buffer) {
            Event::Message {
                ctx: _ctx,
                sender: _ch,
                m,
            } => {
                match m {
                    Message::NewclientRequest(m) => {
                        // FIXME:
                        tcpip_ch = Some(Channel::from_handle(OwnedHandle::from_raw(m.handle())));
                        let tcpip_ch2 =
                            Some(Channel::from_handle(OwnedHandle::from_raw(m.handle())));
                        mainloop
                            .add_channel(tcpip_ch2.unwrap(), Context::Tcpip)
                            .unwrap();
                    }
                    Message::Tx(tx) => {
                        trace!("sending {} bytes", tx.payload().len());
                        let buffer_index =
                            transmitq_buffers.pop_free().expect("no free tx buffers");
                        let vaddr = transmitq_buffers.vaddr(buffer_index);
                        let paddr = transmitq_buffers.paddr(buffer_index);

                        unsafe {
                            vaddr.as_mut_ptr::<VirtioNetModernHeader>().write(
                                VirtioNetModernHeader {
                                    flags: 0,
                                    hdr_len: 0,
                                    gso_type: 0,
                                    gso_size: 0,
                                    checksum_start: 0,
                                    checksum_offset: 0,
                                    // num_buffer: 0,
                                },
                            );
                        }

                        let header_len = size_of::<VirtioNetModernHeader>();
                        unsafe {
                            let buf = core::slice::from_raw_parts_mut(
                                vaddr.add(header_len).as_mut_ptr(),
                                dma_buf_len - header_len,
                            );
                            buf[..tx.payload().len()].copy_from_slice(tx.payload().as_slice());
                        }

                        let chain = &[
                            VirtqDescBuffer::ReadOnlyFromDevice {
                                paddr,
                                len: header_len,
                            },
                            VirtqDescBuffer::ReadOnlyFromDevice {
                                paddr: paddr.add(header_len),
                                len: tx.payload().len(),
                            },
                        ];

                        transmitq.enqueue(chain);
                        transmitq.notify(&mut *transport);
                    }
                    _ => {
                        warn!("unepxected message: {:?}", m);
                    }
                }
            }
            Event::Interrupt {
                ctx: _ctx,
                interrupt,
            } => {
                loop {
                    let status = transport.read_isr_status();
                    if !status.queue_intr() {
                        break;
                    }

                    while let Some(VirtqUsedChain { descs, total_len }) = receiveq.pop_used() {
                        debug_assert!(descs.len() == 1);
                        let mut remaining = total_len;
                        for desc in descs {
                            let VirtqDescBuffer::WritableFromDevice { paddr, len } = desc else {
                                panic!("unexpected desc");
                            };

                            let read_len = core::cmp::min(len, remaining);
                            remaining -= read_len;

                            let buffer_index = receiveq_buffers
                                .paddr_to_index(paddr)
                                .expect("invalid paddr");
                            let vaddr = receiveq_buffers.vaddr(buffer_index);
                            let header_len = size_of::<VirtioNetModernHeader>();
                            let data = unsafe {
                                core::slice::from_raw_parts(
                                    vaddr.as_ptr::<u8>().add(header_len),
                                    read_len,
                                )
                            };

                            trace!("received {} bytes", data.len());
                            if let Some(tcpip_ch) = &tcpip_ch {
                                // FIXME:
                                let mut tmpbuf = [0; 1514];
                                tmpbuf[..data.len()].copy_from_slice(&data);

                                let rx = ethernet_device::Rx {
                                    payload: BytesField::new(tmpbuf, data.len() as u16),
                                };
                                if let Err(err) = tcpip_ch.send_with_buffer(&mut buffer, rx) {
                                    warn!("failed to send rx: {:?}", err);
                                }
                            } else {
                                warn!("no tcpip ch, droppping packet...");
                            }

                            receiveq_buffers.push_free(buffer_index);
                        }
                    }

                    while let Some(VirtqUsedChain { descs, total_len }) = transmitq.pop_used() {
                        let VirtqDescBuffer::ReadOnlyFromDevice { paddr, len } = descs[0] else {
                            panic!("unexpected desc");
                        };
                        let buffer_index = transmitq_buffers
                            .paddr_to_index(paddr)
                            .expect("invalid paddr");
                        transmitq_buffers.push_free(buffer_index);
                    }

                    while let Some(buffer_index) = receiveq_buffers.pop_free() {
                        let chain = &[VirtqDescBuffer::WritableFromDevice {
                            paddr: receiveq_buffers.paddr(buffer_index),
                            len: dma_buf_len,
                        }];

                        receiveq.enqueue(chain);
                    }

                    receiveq.notify(&mut *transport);
                    transport.ack_interrupt(status);
                    interrupt.ack().unwrap();
                }
            }
            _ => {
                warn!("unhandled event");
            }
        }
    }
}
