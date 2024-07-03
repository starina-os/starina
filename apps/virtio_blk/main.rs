#![no_std]
#![no_main]

use ftl_api::folio::MmioFolio;
use ftl_api::prelude::*;
use ftl_api::types::address::PAddr;
use ftl_api::types::address::VAddr;
use ftl_api::types::environ::Device;
use ftl_api_autogen::apps::virtio_blk::Environ;
use ftl_virtio::transports::mmio::VirtioMmio;
use ftl_virtio::transports::VirtioTransport;
use ftl_virtio::virtqueue::align_up;
use ftl_virtio::virtqueue::VirtqDescBuffer;
use ftl_virtio::VIRTIO_DEVICE_TYPE_BLK;

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

fn probe(devices: &[Device], device_type: u32) -> Option<VirtioMmio> {
    for device in devices {
        let base_paddr = PAddr::new(device.reg as usize).unwrap();
        let mmio = MmioFolio::create_pinned(base_paddr, 0x1000).unwrap();

        let mut transport = VirtioMmio::new(mmio);
        match transport.probe() {
            Some(ty) if ty == device_type => {
                return Some(transport);
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

#[ftl_api::main]
pub fn main(env: Environ) {
    info!("starting virtio_blk: {:?}", env.depends.virtio);
    let transport = probe(&env.depends.virtio, VIRTIO_DEVICE_TYPE_BLK).unwrap();
    let mut transport = Box::new(transport) as Box<dyn VirtioTransport>;
    let mut virtqueues = transport.initialize(0, 1).unwrap();

    let mut requestq = virtqueues.get_mut(0).unwrap().take().unwrap();
    let mut requestq_buffers = BufferPool::new(4096, requestq.num_descs() as usize);

    info!("virtio_blk test ----------------------");
    {
        // struct virtio_blk_req {
        //         le32 type;
        //         le32 reserved;
        //         le64 sector;
        //         u8 data[];
        //         u8 status;
        // };
        #[repr(C, packed)]
        struct VirtioBlkReq {
            type_: u32,
            reserved: u32,
            sector: u64,
            data: [u8; 512],
            status: u8,
        }

        // const VIRTIO_BLK_T_IN: u32 = 0;
        const VIRTIO_BLK_T_OUT: u32 = 1;

        let buffer_index = requestq_buffers.pop_free().expect("no free tx buffers");
        let vaddr = requestq_buffers.vaddr(buffer_index);
        let paddr = requestq_buffers.paddr(buffer_index);

        let data = "Hello World from virtio-blk!";

        let req = unsafe { &mut *vaddr.as_mut_ptr::<VirtioBlkReq>() };
        req.type_ = VIRTIO_BLK_T_OUT;
        req.reserved = 0;
        req.sector = 0;

        let data_len = data.as_bytes().len();
        assert!(data_len <= req.data.len());
        req.data[0..data_len].copy_from_slice(data.as_bytes());

        let header_len = size_of::<u32>() + size_of::<u32>() + size_of::<u64>();
        let chain = &[
            VirtqDescBuffer::ReadOnlyFromDevice {
                paddr,
                len: header_len + req.data.len(),
            },
            VirtqDescBuffer::WritableFromDevice {
                paddr: paddr.add(header_len + data.len()),
                len: size_of::<u8>(),
            },
        ];

        info!("chain: {:x?}", chain);
        requestq.enqueue(chain);
        requestq.notify(&mut *transport);
        info!("sent a request");
    }

    loop {}
}
