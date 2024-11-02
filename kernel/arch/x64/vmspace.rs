use core::arch::asm;
use core::mem;

use starina_types::address::PAddr;
use starina_types::address::VAddr;
use starina_types::error::FtlError;
use starina_utils::alignment::is_aligned;

use crate::arch::paddr2vaddr;
use crate::arch::PAGE_SIZE;
use crate::folio::Folio;
use crate::spinlock::SpinLock;

pub const USERSPACE_START: VAddr = VAddr::new(0x0000_000a_0000_0000);
pub const USERSPACE_END: VAddr = VAddr::new(0x0000_000a_ffff_ffff);
const VALLOC_START: VAddr = VAddr::new(0x0000_000b_0000_0000);
const VALLOC_END: VAddr = VAddr::new(0x0000_000b_ffff_ffff);

const ENTRIES_PER_TABLE: usize = 512;
const PTE_P: u64 = 1 << 0;
const PTE_W: u64 = 1 << 1;
const PTE_G: u64 = 1 << 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct Entry(u64);

impl Entry {
    pub const fn new(paddr: PAddr, flags: u64) -> Self {
        assert!(is_aligned(paddr.as_usize(), PAGE_SIZE));

        Self(paddr.as_usize() as u64 | flags)
    }

    pub fn is_present(&self) -> bool {
        self.0 & PTE_P != 0
    }

    pub fn paddr(&self) -> PAddr {
        PAddr::new((self.0 & 0x0000_ffff_ffff_f000) as usize)
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

struct PageTable {
    pml4: Folio,
}

impl PageTable {
    pub fn new() -> Result<PageTable, FtlError> {
        let pml4 = Folio::alloc(size_of::<Table>())?;
        Ok(PageTable { pml4 })
    }

    pub fn cr3(&self) -> u64 {
        self.pml4.paddr().as_usize() as u64
    }

    pub fn map_kernel_space(&mut self) -> Result<(), FtlError> {
        // Kernel memory
        self.map_range(
            VAddr::new(0xffff_8000_0000_0000),
            PAddr::new(0x0000_0000_0000_0000),
            0x8000000,
            PTE_G | PTE_W,
        )?;
        // IO APIC
        // TODO: Disable caching
        self.map_range(
            VAddr::new(0xffff_8000_fec0_0000),
            PAddr::new(0x0000_0000_fec0_0000),
            0x1000,
            PTE_G | PTE_W,
        )?;
        // Local APIC
        // TODO: Disable caching
        self.map_range(
            VAddr::new(0xffff_8000_fee0_0000),
            PAddr::new(0x0000_0000_fee0_0000),
            0x1000,
            PTE_G | PTE_W,
        )?;
        Ok(())
    }

    fn map_range(
        &mut self,
        vaddr: VAddr,
        paddr: PAddr,
        len: usize,
        flags: u64,
    ) -> Result<(), FtlError> {
        assert!(is_aligned(len, PAGE_SIZE));

        for offset in (0..len).step_by(PAGE_SIZE) {
            self.map(vaddr.add(offset), paddr.add(offset), PAGE_SIZE, flags)?;
        }
        Ok(())
    }

    pub fn map(
        &mut self,
        vaddr: VAddr,
        paddr: PAddr,
        len: usize,
        flags: u64,
    ) -> Result<(), FtlError> {
        // trace!("map: {:08x} -> {:08x}", vaddr.as_usize(), paddr.as_usize());
        assert!(is_aligned(vaddr.as_usize(), PAGE_SIZE));
        assert!(is_aligned(paddr.as_usize(), PAGE_SIZE));
        assert!(is_aligned(len, PAGE_SIZE));

        for offset in (0..len).step_by(PAGE_SIZE) {
            self.map_4kb(vaddr.add(offset), paddr.add(offset), flags)?;
        }

        // FIXME: Invalidate TLB

        Ok(())
    }

    fn paddr2table(&mut self, paddr: PAddr) -> Result<&mut Table, FtlError> {
        let vaddr = paddr2vaddr(paddr)?;
        Ok(unsafe { &mut *vaddr.as_mut_ptr() })
    }

    fn map_4kb(&mut self, vaddr: VAddr, paddr: PAddr, flags: u64) -> Result<(), FtlError> {
        assert!(is_aligned(vaddr.as_usize(), PAGE_SIZE));
        assert!(is_aligned(paddr.as_usize(), PAGE_SIZE));

        // println!(
        //     "map_4kb: {:08x} -> {:08x}",
        //     vaddr.as_usize(),
        //     paddr.as_usize()
        // );
        let mut table = self.paddr2table(self.pml4.paddr())?;
        for level in (1..=3).rev() {
            let entry = table.get_mut_by_vaddr(vaddr, level);
            if !entry.is_present() {
                // Allocate a new table.
                let new_table = Folio::alloc(size_of::<Table>())?;
                *entry = Entry::new(new_table.paddr(), PTE_P);

                // TODO: Initialize the new table with zeros.

                // This vmspace object owns the allocated folio.
                // TODO: deallocate on Drop
                mem::forget(new_table);
            }

            // Traverse to the next table.
            let next_table_paddr = entry.paddr();
            table = self.paddr2table(next_table_paddr)?;
        }

        let entry = table.get_mut_by_vaddr(vaddr, 0);
        if entry.is_present() {
            return Err(FtlError::AlreadyMapped);
        }

        *entry = Entry::new(paddr, flags | PTE_P);
        Ok(())
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

    pub fn alloc(&mut self, len: usize) -> Result<VAddr, FtlError> {
        let vaddr = self.next_vaddr;
        if vaddr.add(len) > VALLOC_END {
            return Err(FtlError::TooLarge);
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
    cr3: u64,
    mutable: SpinLock<Mutable>,
}

impl VmSpace {
    pub fn new() -> Result<VmSpace, FtlError> {
        let mut table = PageTable::new()?;
        table.map_kernel_space()?;

        let cr3 = table.cr3();
        Ok(VmSpace {
            cr3,
            mutable: SpinLock::new(Mutable {
                table,
                valloc: VAlloc::new(),
            }),
        })
    }

    pub fn map_fixed(&self, vaddr: VAddr, paddr: PAddr, len: usize) -> Result<(), FtlError> {
        let mut mutable = self.mutable.lock();
        mutable.table.map_range(vaddr, paddr, len, PTE_W | PTE_P)?;
        Ok(())
    }

    pub fn map_anywhere(&self, paddr: PAddr, len: usize) -> Result<VAddr, FtlError> {
        let mut mutable = self.mutable.lock();
        let vaddr = mutable.valloc.alloc(len)?;
        mutable.table.map_range(vaddr, paddr, len, PTE_W | PTE_P)?;
        Ok(vaddr)
    }

    pub fn switch(&self) {
        unsafe {
            asm!("mov cr3, {}", in(reg) self.cr3);
        }
    }
}
