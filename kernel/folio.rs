use crate::memory::AllocPagesError;
use crate::memory::AllocatedPages;

pub struct Folio {
    #[allow(dead_code)]
    pages: AllocatedPages,
}

impl Folio {
    pub fn alloc(len: usize) -> Result<Folio, AllocPagesError> {
        let pages = AllocatedPages::alloc(len)?;
        Ok(Folio { pages })
    }

    pub fn allocated_pages(&self) -> &AllocatedPages {
        &self.pages
    }

    pub fn allocated_pages_mut(&mut self) -> &mut AllocatedPages {
        &mut self.pages
    }
}
