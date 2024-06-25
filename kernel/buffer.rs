use crate::memory::AllocPagesError;
use crate::memory::AllocatedPages;

pub struct Buffer {
    #[allow(dead_code)]
    pages: AllocatedPages,
}

impl Buffer {
    pub fn alloc(len: usize) -> Result<Buffer, AllocPagesError> {
        let pages = AllocatedPages::alloc(len)?;
        Ok(Buffer { pages })
    }

    pub fn allocated_pages(&self) -> &AllocatedPages {
        &self.pages
    }

    pub fn allocated_pages_mut(&mut self) -> &mut AllocatedPages {
        &mut self.pages
    }
}
