#![no_std]
#![no_main]

ftl_api::autogen!();

use ftl_api::channel::Channel;
use ftl_api::environ::Environ;
use ftl_api::folio::MmioFolio;
use ftl_api::interrupt::Interrupt;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_api::types::address::PAddr;
use ftl_api::types::environ::Device;
use ftl_api::types::interrupt::Irq;
use ftl_autogen::idl::ethernet_device;
use ftl_autogen::idl::Message;
use ftl_driver_utils::buffer_pool::BufferPool;
use ftl_virtio::transports::mmio::VirtioMmio;
use ftl_virtio::transports::VirtioTransport;
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

fn probe(devices: &[Device], device_type: u32) -> Option<(VirtioMmio, Irq)> {
    for device in devices {
        let base_paddr = PAddr::new(device.reg as usize);
        let mmio = MmioFolio::create_pinned(base_paddr, 0x1000).unwrap();

        let mut transport = VirtioMmio::new(mmio);
        match transport.probe() {
            Some(ty) if ty == device_type => {
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

#[derive(Debug)]
enum Context {
    Startup,
    Interrupt,
    Tcpip,
}

#[no_mangle]
pub fn main(mut env: Environ) {
    info!("starting");
    let (mut transport, irq) =
        probe(env.devices("virtio,mmio").unwrap(), VIRTIO_DEVICE_TYPE_NET).unwrap();
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
        .add_channel(env.take_channel("dep:startup").unwrap(), Context::Startup)
        .unwrap();
    mainloop
        .add_interrupt(interrupt, Context::Interrupt)
        .unwrap();

    let mut tcpip_sender = None;
    loop {
        match mainloop.next() {
            Event::Message {
                ctx: Context::Startup,
                message: Message::NewClient(m),
                ..
            } => {
                let tcpip_ch = m.handle.take::<Channel>().unwrap();
                let (sender, receiver) = tcpip_ch.split();
                tcpip_sender = Some(sender.clone());

                mainloop
                    .add_channel((sender, receiver), Context::Tcpip)
                    .unwrap();
            }
            Event::Message {
                ctx: Context::Tcpip,
                message: Message::Tx(m),
                ..
            } => {
                trace!("sending {} bytes", m.payload.len());
                let buffer_index = transmitq_buffers.pop_free().expect("no free tx buffers");
                let vaddr = transmitq_buffers.vaddr(buffer_index);
                let paddr = transmitq_buffers.paddr(buffer_index);

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
                        dma_buf_len - header_len,
                    );
                    buf[..m.payload.len()].copy_from_slice(m.payload.as_slice());
                }

                let chain = &[
                    VirtqDescBuffer::ReadOnlyFromDevice {
                        paddr,
                        len: header_len,
                    },
                    VirtqDescBuffer::ReadOnlyFromDevice {
                        paddr: paddr.add(header_len),
                        len: m.payload.len(),
                    },
                ];

                transmitq.enqueue(chain);
                transmitq.notify(&mut *transport);
            }
            Event::Interrupt {
                ctx: Context::Interrupt,
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
                            if let Some(tcpip_sender) = &tcpip_sender {
                                let rx = ethernet_device::Rx {
                                    payload: data.try_into().unwrap(),
                                };
                                if let Err(err) = tcpip_sender.send(rx) {
                                    warn!("failed to send rx: {:?}", err);
                                }
                            } else {
                                warn!("no tcpip ch, droppping packet...");
                            }

                            receiveq_buffers.push_free(buffer_index);
                        }
                    }

                    while let Some(VirtqUsedChain { descs, .. }) = transmitq.pop_used() {
                        let VirtqDescBuffer::ReadOnlyFromDevice { paddr, .. } = descs[0] else {
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
            ev => {
                warn!("unhandled event: {:?}", ev);
            }
        }
    }
}
