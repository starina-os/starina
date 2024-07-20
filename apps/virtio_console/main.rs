#![no_std]
#![no_main]

use ftl_api::folio::MmioFolio;
use ftl_api::interrupt::Interrupt;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_api::types::address::PAddr;
use ftl_api::types::address::VAddr;
use ftl_api::types::environ::Device;
use ftl_api::types::interrupt::Irq;
use ftl_api::types::message::MessageBuffer;
use ftl_api_autogen::apps::virtio_console::Environ;
use ftl_api_autogen::apps::virtio_console::Message;
use ftl_virtio::transports::mmio::VirtioMmio;
use ftl_virtio::transports::VirtioTransport;
use ftl_virtio::virtqueue::align_up;
use ftl_virtio::virtqueue::VirtqDescBuffer;
use ftl_virtio::virtqueue::VirtqUsedChain;
use ftl_virtio::VIRTIO_DEVICE_TYPE_CONSOLE;

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
}

#[ftl_api::main]
pub fn main(mut env: Environ) {
    info!("starting virtio_console");
    let mut buffer = MessageBuffer::new();

    let (transport, irq) = probe(&env.depends.virtio, VIRTIO_DEVICE_TYPE_CONSOLE).unwrap();
    let interrupt = Interrupt::create(irq).unwrap();
    let mut transport = Box::new(transport) as Box<dyn VirtioTransport>;
    let mut virtqueues = transport.initialize(0, 2).unwrap();

    let mut transmitq = virtqueues.get_mut(1).unwrap().take().unwrap();
    let mut receiveq = virtqueues.get_mut(0).unwrap().take().unwrap();
    let dma_buf_len = 4096;
    let mut receiveq_buffers = BufferPool::new(dma_buf_len, receiveq.num_descs() as usize);
    let mut transmitq_buffers = BufferPool::new(dma_buf_len, transmitq.num_descs() as usize);

    info!("receiveq.num_descs = {}", receiveq.num_descs());
    // Fill the receive queue with buffers.
    while let Some(buffer_index) = receiveq_buffers.pop_free() {
        let chain = &[VirtqDescBuffer::WritableFromDevice {
            paddr: receiveq_buffers.paddr(buffer_index),
            len: dma_buf_len,
        }];

        receiveq.enqueue(chain);
    }
    receiveq.notify(&mut *transport);

    info!("virtio_console test ----------------------");
    {
        let buffer_index = transmitq_buffers.pop_free().expect("no free tx buffers");
        let vaddr = transmitq_buffers.vaddr(buffer_index);
        let paddr = transmitq_buffers.paddr(buffer_index);

        let data = "Hello World from virtio-console!\r\n";

        let dma_buf =
            unsafe { core::slice::from_raw_parts_mut(&mut *vaddr.as_mut_ptr::<u8>(), dma_buf_len) };

        let data_len = data.as_bytes().len();
        assert!(data_len <= dma_buf.len());
        dma_buf[0..data_len].copy_from_slice(data.as_bytes());

        let chain = &[VirtqDescBuffer::ReadOnlyFromDevice {
            paddr,
            len: data_len,
        }];

        info!("chain: {:x?}", chain);
        transmitq.enqueue(chain);
        transmitq.notify(&mut *transport);
        info!("sent a request");
    }

    let mut mainloop = Mainloop::<Context, Message>::new().unwrap();
    mainloop
        .add_channel(env.autopilot_ch.take().unwrap(), Context::Autopilot)
        .unwrap();
    mainloop
        .add_interrupt(interrupt, Context::Interrupt)
        .unwrap();

    loop {
        match mainloop.next(&mut buffer) {
            Event::Interrupt { ctx: _ctx, interrupt } => {
                let status = transport.read_isr_status();
                info!("got interrupt!: status={:?}", status.0);
                transport.ack_interrupt(status);

                while let Some(VirtqUsedChain { descs, total_len }) = receiveq.pop_used() {
                    info!("interrupt: total_len={}", total_len);
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
                        let data =
                            unsafe { core::slice::from_raw_parts(vaddr.as_ptr::<u8>(), read_len) };
                        info!("received: {:?}", core::str::from_utf8(data).unwrap());
                        receiveq_buffers.push_free(buffer_index);
                    }
                }

                while let Some(buffer_index) = receiveq_buffers.pop_free() {
                    let chain = &[VirtqDescBuffer::WritableFromDevice {
                        paddr: receiveq_buffers.paddr(buffer_index),
                        len: dma_buf_len,
                    }];

                    receiveq.enqueue(chain);
                }
                receiveq.notify(&mut *transport);

                interrupt.ack().unwrap();
            }
            _ => {
                warn!("unhandled event");
            }
        }
    }
}
