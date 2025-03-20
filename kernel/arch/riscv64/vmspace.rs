use core::arch::asm;
use core::mem;

use starina_types::address::PAddr;
use starina_types::address::VAddr;
use starina_types::error::ErrorCode;
use starina_types::vmspace::PageProtect;
use starina_utils::alignment::is_aligned;

use crate::arch::PAGE_SIZE;
use crate::arch::paddr2vaddr;
use crate::folio::Folio;
use crate::spinlock::SpinLock;

const ENTRIES_PER_TABLE: usize = 512;
const PPN_SHIFT: usize = 12;

const PTE_V: u64 = 1 << 0;
const PTE_R: u64 = 1 << 1;
const PTE_W: u64 = 1 << 2;
const PTE_X: u64 = 1 << 3;
const PTE_U: u64 = 1 << 4;
const PTE_PPN_SHIFT: usize = 10;

const SATP_MODE_SV48: u64 = 9 << 60;

pub const USERSPACE_START: VAddr = VAddr::new(0x0000_000a_0000_0000);
pub const USERSPACE_END: VAddr = VAddr::new(0x0000_000a_ffff_ffff);
const VALLOC_START: VAddr = VAddr::new(0x0000_000b_0000_0000);
const VALLOC_END: VAddr = VAddr::new(0x0000_000b_ffff_ffff);

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
    pub fn get_mut_by_vaddr(&mut self, vaddr: VAddr, level: usize) -> &mut Entry {
        let index = (vaddr.as_usize() >> (12 + 9 * level)) & 0x1ff;
        &mut self.0[index]
    }
}

fn is_large_page_aligned(
    vaddr: VAddr,
    paddr: PAddr,
    remaining: usize,
    level: usize,
) -> Option<usize> {
    let page_size = match level {
        1 => 2 * 1024 * 1024,        // 2MiB pages (level 1)
        2 => 1 * 1024 * 1024 * 1024, // 1GiB pages (level 2)
        3 => return None,            // Not supported in Sv48
        _ => unreachable!(),
    };

    if page_size > remaining {
        return None;
    }

    if !is_aligned(vaddr.as_usize(), page_size) {
        return None;
    }

    if !is_aligned(paddr.as_usize(), page_size) {
        return None;
    }

    Some(page_size)
}

struct PageTable {
    l0_table: Folio,
}

impl PageTable {
    pub fn new() -> Result<PageTable, ErrorCode> {
        let l0_table = Folio::alloc(size_of::<Table>())?;
        let mut table = PageTable { l0_table };
        table.map_kernel_space()?;
        Ok(table)
    }

    // FIXME: Move to machine-specific code.
    pub fn map_kernel_space(&mut self) -> Result<(), ErrorCode> {
        // Kernel memory
        self.do_map(
            VAddr::new(0x8020_0000),
            PAddr::new(0x8020_0000),
            0x8ff00000 - 0x8020_0000,
            PageProtect::READABLE | PageProtect::WRITEABLE | PageProtect::EXECUTABLE,
            true,
        )?;
        // PLIC
        self.do_map(
            VAddr::new(0x0c00_0000),
            PAddr::new(0x0c00_0000),
            0x400000,
            PageProtect::READABLE | PageProtect::WRITEABLE,
            true,
        )?;
        // UART
        self.do_map(
            VAddr::new(0x1000_0000),
            PAddr::new(0x1000_0000),
            0x1000,
            PageProtect::READABLE | PageProtect::WRITEABLE,
            true,
        )?;
        Ok(())
    }

    pub fn map(
        &mut self,
        vaddr: VAddr,
        paddr: PAddr,
        len: usize,
        prot: PageProtect,
    ) -> Result<(), ErrorCode> {
        self.do_map(vaddr, paddr, len, prot, false)
    }

    pub fn do_map(
        &mut self,
        vaddr: VAddr,
        paddr: PAddr,
        len: usize,
        prot: PageProtect,
        allow_large_pages: bool,
    ) -> Result<(), ErrorCode> {
        // trace!("map: {:08x} -> {:08x}", vaddr.as_usize(), paddr.as_usize());
        assert!(is_aligned(vaddr.as_usize(), PAGE_SIZE));
        assert!(is_aligned(paddr.as_usize(), PAGE_SIZE));
        assert!(is_aligned(len, PAGE_SIZE));

        let mut offset = 0;
        while offset < len {
            let remaining = len - offset;
            let page_size = self.map_page(
                vaddr.add(offset),
                paddr.add(offset),
                prot,
                remaining,
                allow_large_pages,
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
        vaddr: VAddr,
        paddr: PAddr,
        prot: PageProtect,
        remaining: usize,
        allow_large_pages: bool,
    ) -> Result<usize, ErrorCode> {
        assert!(is_aligned(vaddr.as_usize(), PAGE_SIZE));
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
                vaddr.as_usize(),
                paddr.as_usize()
            );
            return Err(ErrorCode::InvalidArg);
        }

        let mut table = self.paddr2table(self.l0_table.paddr())?;
        for level in (1..=3).rev() {
            let entry = table.get_mut_by_vaddr(vaddr, level);
            if !entry.is_valid() {
                if allow_large_pages {
                    if let Some(page_size) = is_large_page_aligned(vaddr, paddr, remaining, level) {
                        // Allocate a large page.
                        *entry = Entry::new(paddr, PTE_V | leaf_flags);
                        return Ok(page_size);
                    }
                }

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

        let entry = table.get_mut_by_vaddr(vaddr, 0);
        if entry.is_valid() {
            return Err(ErrorCode::AlreadyMapped);
        }

        *entry = Entry::new(paddr, leaf_flags);
        Ok(4096)
    }
}

struct VAlloc {
    next_vaddr: VAddr,
}

impl VAlloc {
    pub const fn new() -> VAlloc {
        VAlloc {
            next_vaddr: VALLOC_START,
        }
    }

    pub fn alloc(&mut self, len: usize) -> Result<VAddr, ErrorCode> {
        let vaddr = self.next_vaddr;
        if vaddr.add(len) > VALLOC_END {
            return Err(ErrorCode::TooLarge);
        }

        self.next_vaddr = vaddr.add(len);
        Ok(vaddr)
    }
}

struct Mutable {
    table: PageTable,
    valloc: VAlloc,
}

pub struct VmSpace {
    mutable: SpinLock<Mutable>,
    satp: u64,
}

impl VmSpace {
    pub fn new() -> Result<VmSpace, ErrorCode> {
        let mut table = PageTable::new()?;
        let table_paddr = table.l0_table.paddr().as_usize() as u64;
        let satp = SATP_MODE_SV48 | (table_paddr >> PPN_SHIFT);
        Ok(VmSpace {
            satp,
            mutable: SpinLock::new(Mutable {
                table,
                valloc: VAlloc::new(),
            }),
        })
    }

    pub fn map_fixed(
        &self,
        vaddr: VAddr,
        paddr: PAddr,
        len: usize,
        prot: PageProtect,
    ) -> Result<(), ErrorCode> {
        self.mutable.lock().table.map(vaddr, paddr, len, prot)
    }

    pub fn map_anywhere(
        &self,
        paddr: PAddr,
        len: usize,
        prot: PageProtect,
    ) -> Result<VAddr, ErrorCode> {
        assert!(is_aligned(len, PAGE_SIZE));

        let mut mutable = self.mutable.lock();
        let vaddr = mutable.valloc.alloc(len)?;
        mutable.table.map(vaddr, paddr, len, prot)?;
        Ok(vaddr)
    }

    pub fn switch(&self) {
        unsafe {
            // Do sfeence.vma before and even before switching the page
            // table to ensure all changes prior to this switch are visible.
            //
            // (The RISC-V Instruction Set Manual Volume II, Version 1.10, p. 58)
            asm!("
                sfence.vma
                csrw satp, {}
                sfence.vma
            ", in(reg) self.satp);
        }
    }
}
