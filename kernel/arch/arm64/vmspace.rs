use core::arch::asm;
use core::mem;

use ftl_types::address::PAddr;
use ftl_types::address::VAddr;
use ftl_types::error::FtlError;
use ftl_types::vmspace::PageProtect;
use ftl_utils::alignment::is_aligned;

use crate::arch::paddr2vaddr;
use crate::arch::PAGE_SIZE;
use crate::folio::Folio;
use crate::spinlock::SpinLock;

const ENTRIES_PER_TABLE: usize = 512;

#[repr(u64)]
enum PteType {
    TableOrPage = 0b11,
}

// https://developer.arm.com/documentation/102376/0200/Permissions
const PTE_AP_USER: u64 = 1 << 6;
const PTE_AP_READONLY: u64 = 1 << 7;

const PTE_PADDR_MASK: u64 = 0x0000fffffffff000;

pub const USERSPACE_START: VAddr = VAddr::new(0x0000_000a_0000_0000);
pub const USERSPACE_END: VAddr = VAddr::new(0x0000_000a_ffff_ffff);
const VALLOC_START: VAddr = VAddr::new(0x0000_000b_0000_0000);
const VALLOC_END: VAddr = VAddr::new(0x0000_000b_ffff_ffff);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct Entry(u64);

impl Entry {
    pub fn new(paddr: PAddr, pte_type: PteType, flags: u64) -> Self {
        assert!(is_aligned(paddr.as_usize(), PAGE_SIZE));
        assert!(
            (paddr.as_usize() & 0xffff_0000_0000_0000) == 0,
            "ttbr1 is not supported"
        );

        Self((paddr.as_usize() as u64) | flags | (pte_type as u64))
    }

    pub fn is_invalid(&self) -> bool {
        self.0 & 1 == 0
    }

    pub fn paddr(&self) -> PAddr {
        PAddr::new((self.0 & PTE_PADDR_MASK) as usize)
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
    l0_table: Folio,
}

impl PageTable {
    pub fn new() -> Result<PageTable, FtlError> {
        let l0_table = Folio::alloc(size_of::<Table>())?;
        Ok(PageTable { l0_table })
    }

    pub fn map_kernel_space(&mut self) -> Result<(), FtlError> {
        // Kernel memory
        self.map_range(
            VAddr::new(0x4000_0000),
            PAddr::new(0x4000_0000),
            0x1000_0000,
            PageProtect::READABLE | PageProtect::WRITABLE | PageProtect::EXECUTABLE,
        )?;
        // GIC (TODO: use addresses from device tree)
        self.map_range(
            VAddr::new(0x0800_0000),
            PAddr::new(0x0800_0000),
            0x1000,
            PageProtect::READABLE | PageProtect::WRITABLE,
        )?;
        self.map_range(
            VAddr::new(0x0801_0000),
            PAddr::new(0x0801_0000),
            0x1000,
            PageProtect::READABLE | PageProtect::WRITABLE,
        )?;
        // UART
        self.map_range(
            VAddr::new(0x0900_0000),
            PAddr::new(0x0900_0000),
            0x1000,
            PageProtect::READABLE | PageProtect::WRITABLE,
        )?;
        Ok(())
    }

    fn map_range(
        &mut self,
        vaddr: VAddr,
        paddr: PAddr,
        len: usize,
        prot: PageProtect,
    ) -> Result<(), FtlError> {
        assert!(is_aligned(len, PAGE_SIZE));

        for offset in (0..len).step_by(PAGE_SIZE) {
            self.map(vaddr.add(offset), paddr.add(offset), PAGE_SIZE, prot)?;
        }
        Ok(())
    }

    pub fn map(
        &mut self,
        vaddr: VAddr,
        paddr: PAddr,
        len: usize,
        prot: PageProtect,
    ) -> Result<(), FtlError> {
        // trace!("map: {:08x} -> {:08x}", vaddr.as_usize(), paddr.as_usize());
        assert!(is_aligned(vaddr.as_usize(), PAGE_SIZE));
        assert!(is_aligned(paddr.as_usize(), PAGE_SIZE));
        assert!(is_aligned(len, PAGE_SIZE));

        for offset in (0..len).step_by(PAGE_SIZE) {
            self.map_4kb(vaddr.add(offset), paddr.add(offset), prot)?;
        }

        unsafe {
            asm!("dsb ishst");
        }

        for offset in (0..len).step_by(PAGE_SIZE) {
            unsafe {
                asm!(
                    "tlbi vae1is, {}",
                    in(reg) vaddr.add(offset).as_usize(),
                );
            }
        }

        unsafe {
            asm!("isb");
        }

        Ok(())
    }

    fn paddr2table(&mut self, paddr: PAddr) -> Result<&mut Table, FtlError> {
        let vaddr = paddr2vaddr(paddr)?;
        Ok(unsafe { &mut *vaddr.as_mut_ptr() })
    }

    fn map_4kb(&mut self, vaddr: VAddr, paddr: PAddr, prot: PageProtect) -> Result<(), FtlError> {
        assert!(is_aligned(vaddr.as_usize(), PAGE_SIZE));
        assert!(is_aligned(paddr.as_usize(), PAGE_SIZE));

        let mut table = self.paddr2table(self.l0_table.paddr())?;
        for level in (1..=3).rev() {
            let entry = table.get_mut_by_vaddr(vaddr, level);
            if entry.is_invalid() {
                // Allocate a new table.
                let new_table = Folio::alloc(size_of::<Table>())?;
                *entry = Entry::new(new_table.paddr(), PteType::TableOrPage, 1 << 10);

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
        if !entry.is_invalid() {
            return Err(FtlError::AlreadyMapped);
        }

        let mut flags = 1 << 10; // FIXME: Why?
        if !prot.contains(PageProtect::WRITABLE) {
            flags |= PTE_AP_READONLY;
        }

        if !prot.contains(PageProtect::EXECUTABLE) {
            // TODO:
        }

        if prot.contains(PageProtect::USER) {
            flags |= PTE_AP_USER;
        }

        *entry = Entry::new(paddr, PteType::TableOrPage, flags);
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
    mutable: SpinLock<Mutable>,
    ttbr0: u64,
}

impl VmSpace {
    pub fn new() -> Result<VmSpace, FtlError> {
        let mut table = PageTable::new()?;
        table.map_kernel_space()?;

        let table_paddr = table.l0_table.paddr().as_usize() as u64;
        let ttbr0 = table_paddr;
        Ok(VmSpace {
            ttbr0,
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
    ) -> Result<(), FtlError> {
        self.mutable.lock().table.map(vaddr, paddr, len, prot)
    }

    pub fn map_anywhere(
        &self,
        paddr: PAddr,
        len: usize,
        prot: PageProtect,
    ) -> Result<VAddr, FtlError> {
        assert!(is_aligned(len, PAGE_SIZE));

        let mut mutable = self.mutable.lock();
        let vaddr = mutable.valloc.alloc(len)?;
        mutable.table.map(vaddr, paddr, len, prot)?;
        Ok(vaddr)
    }

    pub fn switch(&self) {
        unsafe {
            asm!(
                r#"
                    msr ttbr0_el1, {ttbr0}
                    isb
                    tlbi vmalle1is
                    dsb ish
                    isb
                "#,
                ttbr0 = in(reg) self.ttbr0,
            );
        }
    }
}
