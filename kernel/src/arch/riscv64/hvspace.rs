use core::mem;

use starina::address::GPAddr;
use starina::address::PAddr;
use starina::error::ErrorCode;
use starina_types::vmspace::PageProtect;
use starina_utils::alignment::is_aligned;

use super::paddr2vaddr;
use crate::arch::PAGE_SIZE;
use crate::folio::Folio;
use crate::spinlock::SpinLock;

const HGATP_MODE_SV48: u64 = 9 << 60;
const ENTRIES_PER_TABLE: usize = 512;
const PPN_SHIFT: usize = 12;
const PTE_V: u64 = 1 << 0;
const PTE_R: u64 = 1 << 1;
const PTE_W: u64 = 1 << 2;
const PTE_X: u64 = 1 << 3;
const PTE_U: u64 = 1 << 4;
const PTE_PPN_SHIFT: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct Entry(u64);

impl Entry {
    pub fn new(paddr: PAddr, flags: u64) -> Self {
        assert!(is_aligned(paddr.as_usize(), PAGE_SIZE));

        let ppn = paddr.as_usize() as u64 >> PPN_SHIFT;
        Self(ppn << PTE_PPN_SHIFT | flags)
    }

    pub fn is_valid(&self) -> bool {
        self.0 & PTE_V != 0
    }

    pub fn is_leaf(&self) -> bool {
        self.0 & (PTE_R | PTE_W | PTE_X) != 0
    }

    pub fn ppn(&self) -> u64 {
        self.0 >> PTE_PPN_SHIFT
    }

    pub fn paddr(&self) -> PAddr {
        let raw = self.ppn() << PPN_SHIFT;
        PAddr::new(raw as usize)
    }
}

#[repr(transparent)]
struct Table([Entry; ENTRIES_PER_TABLE]);

impl Table {
    pub fn get_mut_by_gpaddr(&mut self, gpaddr: GPAddr, level: usize) -> &mut Entry {
        let index = (gpaddr.as_usize() >> (12 + 9 * level)) & 0x1ff;
        &mut self.0[index]
    }
}

struct HvPageTable {
    l0_table: Folio,
}

impl HvPageTable {
    pub fn new() -> Result<HvPageTable, ErrorCode> {
        let l0_table = Folio::alloc(size_of::<Table>())?;
        let mut table = HvPageTable { l0_table };
        Ok(table)
    }

    pub fn map(
        &mut self,
        gpaddr: GPAddr,
        paddr: PAddr,
        len: usize,
        prot: PageProtect,
    ) -> Result<(), ErrorCode> {
        self.do_map(gpaddr, paddr, len, prot)
    }

    pub fn do_map(
        &mut self,
        gpaddr: GPAddr,
        paddr: PAddr,
        len: usize,
        prot: PageProtect,
    ) -> Result<(), ErrorCode> {
        trace!(
            "hvspace map: {:08x} -> {:08x}",
            gpaddr.as_usize(),
            paddr.as_usize()
        );
        assert!(is_aligned(gpaddr.as_usize(), PAGE_SIZE));
        assert!(is_aligned(paddr.as_usize(), PAGE_SIZE));
        assert!(is_aligned(len, PAGE_SIZE));
        assert!(gpaddr.checked_add(len).is_some());
        assert!(paddr.checked_add(len).is_some());

        let mut offset = 0;
        while offset < len {
            let remaining = len - offset;
            let page_size = self.map_page(
                gpaddr.checked_add(offset).unwrap(),
                paddr.add(offset),
                prot,
                remaining,
            )?;
            offset += page_size;
        }

        // FIXME: Invalidate TLB
        Ok(())
    }

    fn paddr2table(&mut self, paddr: PAddr) -> Result<&mut Table, ErrorCode> {
        let vaddr = paddr2vaddr(paddr)?;
        Ok(unsafe { &mut *vaddr.as_mut_ptr() })
    }

    fn map_page(
        &mut self,
        gpaddr: GPAddr,
        paddr: PAddr,
        prot: PageProtect,
        remaining: usize,
    ) -> Result<usize, ErrorCode> {
        assert!(is_aligned(gpaddr.as_usize(), PAGE_SIZE));
        assert!(is_aligned(paddr.as_usize(), PAGE_SIZE));

        let mut leaf_flags = PTE_V;
        if prot.contains(PageProtect::READABLE) {
            leaf_flags |= PTE_R;
        }
        if prot.contains(PageProtect::WRITEABLE) {
            leaf_flags |= PTE_W;
        }
        if prot.contains(PageProtect::EXECUTABLE) {
            leaf_flags |= PTE_X;
        }
        if prot.contains(PageProtect::USER) {
            leaf_flags |= PTE_U;
        }

        if leaf_flags & (PTE_R | PTE_W | PTE_X) == 0 {
            // Invalid leaf entry pattern: this does not mean an inaccessible leaf page,
            // but it is a pointer to the next level table!
            debug_warn!(
                "map_page: {:08x} -> {:08x} has no permissions",
                gpaddr.as_usize(),
                paddr.as_usize()
            );
            return Err(ErrorCode::InvalidArg);
        }

        let mut table = self.paddr2table(self.l0_table.paddr())?;
        for level in (1..=3).rev() {
            let entry = table.get_mut_by_gpaddr(gpaddr, level);
            if !entry.is_valid() {
                // Allocate a new table.
                let new_table = Folio::alloc(size_of::<Table>())?;
                *entry = Entry::new(new_table.paddr(), PTE_V);

                // This vmspace object owns the allocated folio.
                // TODO: deallocate on Drop
                mem::forget(new_table);
            }

            if entry.is_leaf() {
                return Err(ErrorCode::AlreadyMapped);
            }

            // Traverse to the next table.
            let next_table_paddr = entry.paddr();
            table = self.paddr2table(next_table_paddr)?;
        }

        let entry = table.get_mut_by_gpaddr(gpaddr, 0);
        if entry.is_valid() {
            return Err(ErrorCode::AlreadyMapped);
        }

        *entry = Entry::new(paddr, leaf_flags);
        Ok(4096)
    }
}

struct Mutable {
    table: HvPageTable,
}

pub struct HvSpace {
    mutable: SpinLock<Mutable>,
    hgatp: u64,
}

impl HvSpace {
    pub fn new() -> Result<HvSpace, ErrorCode> {
        let table = HvPageTable::new()?;
        let table_paddr = table.l0_table.paddr().as_usize() as u64;
        let mutable = SpinLock::new(Mutable { table });
        let hgatp = HGATP_MODE_SV48 | (table_paddr >> PPN_SHIFT);
        Ok(HvSpace { mutable, hgatp })
    }

    pub(super) fn hgatp(&self) -> u64 {
        self.hgatp
    }

    pub fn map(
        &self,
        gpaddr: GPAddr,
        paddr: PAddr,
        len: usize,
        prot: PageProtect,
    ) -> Result<(), ErrorCode> {
        self.mutable.lock().table.map(gpaddr, paddr, len, prot)
    }
}
