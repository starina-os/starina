use core::mem;

use starina::address::PAddr;
use starina::address::VAddr;
use starina::error::ErrorCode;
use starina_utils::alignment::is_aligned;

use super::PAGE_SIZE;
use super::paddr2vaddr;
use crate::arch::riscv64::vmspace::PPN_SHIFT;
use crate::folio::Folio;

const ENTRIES_PER_TABLE: usize = 512;
const PTE_PPN_SHIFT: usize = 10;

pub(super) const PTE_V: u64 = 1 << 0;
pub(super) const PTE_R: u64 = 1 << 1;
pub(super) const PTE_W: u64 = 1 << 2;
pub(super) const PTE_X: u64 = 1 << 3;
pub(super) const PTE_U: u64 = 1 << 4;

pub(super) const SATP_MODE_SV48: u64 = 9 << 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(super) struct Pte(u64);

impl Pte {
    pub fn new(paddr: PAddr, flags: u64) -> Self {
        assert!(is_aligned(paddr.as_usize(), PAGE_SIZE));

        let ppn = paddr.as_usize() as u64 >> PPN_SHIFT;
        Self(ppn << PTE_PPN_SHIFT | flags)
    }

    pub fn set(&mut self, paddr: PAddr, flags: u64) {
        self.0 = Pte::new(paddr, flags).0;
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

enum TableOrLeaf<'a> {
    Table(&'a mut Table),
    Leaf,
}

#[repr(transparent)]
pub(super) struct Table([Pte; ENTRIES_PER_TABLE]);

impl Table {
    pub fn get_mut(&mut self, index: usize) -> &mut Pte {
        &mut self.0[index]
    }

    fn get_child_table(
        &mut self,
        vaddr: VAddr,
        level: usize,
    ) -> Result<TableOrLeaf<'_>, ErrorCode> {
        let index = get_index(vaddr, level);
        let entry = &mut self.0[index];

        if !entry.is_valid() {
            // Allocate a new table.
            let new_table = Folio::alloc(size_of::<Table>())?;
            *entry = Pte::new(new_table.paddr(), PTE_V);

            // This vmspace object owns the allocated folio.
            // TODO: deallocate on Drop
            mem::forget(new_table);
        }

        if entry.is_leaf() {
            return Ok(TableOrLeaf::Leaf);
        }

        let child_table = paddr2table(entry.paddr())?;
        Ok(TableOrLeaf::Table(child_table))
    }
}

fn get_index(vaddr: VAddr, level: usize) -> usize {
    debug_assert!(level <= 3);
    (vaddr.as_usize() >> (12 + 9 * level)) & 0x1ff
}

fn paddr2table<'a>(paddr: PAddr) -> Result<&'a mut Table, ErrorCode> {
    let vaddr = paddr2vaddr(paddr)?;
    Ok(unsafe { &mut *vaddr.as_mut_ptr() })
}

fn walk_into_last_table(l0_table: &Folio, vaddr: VAddr) -> Result<&mut Table, ErrorCode> {
    let mut table = paddr2table(l0_table.paddr())?;
    for level in (1..=3).rev() {
        table = match table.get_child_table(vaddr, level) {
            Ok(TableOrLeaf::Table(table)) => table,
            Ok(TableOrLeaf::Leaf) => return Err(ErrorCode::AlreadyMapped),
            Err(e) => return Err(e),
        };
    }

    Ok(table)
}

pub(super) struct PteIter<'a> {
    l0_table: &'a Folio,
    current: &'a mut Table,
    index: usize,
    vaddr: VAddr,
    end: VAddr,
}

impl<'a> PteIter<'a> {
    pub fn new(l0_table: &'a Folio, vaddr: VAddr, len: usize) -> Result<Self, ErrorCode> {
        let table = walk_into_last_table(l0_table, vaddr)?;
        Ok(Self {
            l0_table,
            current: table,
            index: get_index(vaddr, 0),
            vaddr,
            end: vaddr.add(len),
        })
    }

    pub fn next_entry(&mut self) -> Result<Option<&mut Pte>, ErrorCode> {
        if self.vaddr >= self.end {
            return Ok(None);
        }

        debug_assert!(self.index <= ENTRIES_PER_TABLE);
        if self.index == ENTRIES_PER_TABLE {
            self.current = walk_into_last_table(self.l0_table, self.vaddr)?;
            self.index = 0;
        }

        let pte = self.current.get_mut(self.index);
        self.index += 1;
        self.vaddr = self.vaddr.add(PAGE_SIZE);
        Ok(Some(pte))
    }
}
