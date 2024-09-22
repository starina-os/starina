use ftl_api::folio::MappedFolio;
use ftl_api::prelude::vec::Vec;
use ftl_api::types::address::PAddr;
use ftl_api::types::address::VAddr;
use ftl_utils::alignment::align_up;

#[derive(Copy, Clone)]
pub struct BufferIndex(usize);

pub struct BufferPool {
    folio: MappedFolio,
    free_indices: Vec<BufferIndex>,
    buffer_size: usize,
    num_buffers: usize,
}

impl BufferPool {
    pub fn new(buffer_size: usize, num_buffers: usize) -> BufferPool {
        let folio = MappedFolio::create(align_up(buffer_size * num_buffers, 4096)).unwrap();
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
