use core::arch::asm;

use starina::address::DAddr;
use starina_types::address::PAddr;
use starina_types::address::VAddr;
use starina_types::error::ErrorCode;
use starina_types::vmspace::PageProtect;
use starina_utils::alignment::is_aligned;

use super::sv48::SATP_MODE_SV48;
use super::sv48::Table;
use crate::arch::riscv64::sv48::PTE_R;
use crate::arch::riscv64::sv48::PTE_U;
use crate::arch::riscv64::sv48::PTE_V;
use crate::arch::riscv64::sv48::PTE_W;
use crate::arch::riscv64::sv48::PTE_X;
use crate::arch::riscv64::sv48::PteIter;
use crate::folio::Folio;
use crate::spinlock::SpinLock;

pub const PAGE_SIZE: usize = 4096;
pub(super) const PPN_SHIFT: usize = 12;

const VALLOC_START: VAddr = VAddr::new(0x0000_000b_0000_0000);
const VALLOC_END: VAddr = VAddr::new(0x0000_000b_ffff_ffff);

pub fn map_daddr(paddr: PAddr) -> Result<DAddr, ErrorCode> {
    // We don't have IOMMU. Device will see the same address as the kernel.
    Ok(DAddr::new(paddr.as_usize()))
}

pub fn unmap_daddr(_daddr: DAddr) -> Result<(), ErrorCode> {
    // We don't do anything in map_daddr. Nothing to unmap.
    Ok(())
}

pub fn vaddr2paddr(vaddr: VAddr) -> Result<PAddr, ErrorCode> {
    // Identical mapping.
    // FIXME:
    Ok(PAddr::new(vaddr.as_usize()))
}

pub fn paddr2vaddr(paddr: PAddr) -> Result<VAddr, ErrorCode> {
    // Identical mapping.
    // FIXME:
    Ok(VAddr::new(paddr.as_usize()))
}

unsafe extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
}

pub fn find_free_ram<F>(paddr: PAddr, len: usize, mut callback: F)
where
    F: FnMut(PAddr, usize),
{
    let _kernel_start = PAddr::new(&raw const __kernel_start as usize);
    let kernel_end = PAddr::new(&raw const __kernel_end as usize);

    // FIXME:
    // if paddr < kernel_start {
    //     let before_len = kernel_start.as_usize() - paddr.as_usize();
    //     callback(paddr, before_len);
    // }

    let end = paddr.add(len);
    if end > kernel_end {
        let after_len = end.as_usize() - kernel_end.as_usize();
        callback(kernel_end, after_len);
    }
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
        self.map(
            VAddr::new(0x8000_0000),
            PAddr::new(0x8000_0000),
            0x90000000 - 0x8000_0000,
            PageProtect::READABLE | PageProtect::WRITEABLE | PageProtect::EXECUTABLE,
        )?;
        // PLIC
        self.map(
            VAddr::new(0x0c00_0000),
            PAddr::new(0x0c00_0000),
            0x400000,
            PageProtect::READABLE | PageProtect::WRITEABLE,
        )?;
        // UART
        self.map(
            VAddr::new(0x1000_0000),
            PAddr::new(0x1000_0000),
            0x1000,
            PageProtect::READABLE | PageProtect::WRITEABLE,
        )?;
        Ok(())
    }

    pub fn map(
        &mut self,
        vaddr: VAddr,
        mut paddr: PAddr,
        len: usize,
        prot: PageProtect,
    ) -> Result<(), ErrorCode> {
        assert!(is_aligned(vaddr.as_usize(), PAGE_SIZE));
        assert!(is_aligned(paddr.as_usize(), PAGE_SIZE));
        assert!(is_aligned(len, PAGE_SIZE));

        let mut flags = PTE_V;
        if prot.contains(PageProtect::READABLE) {
            flags |= PTE_R;
        }
        if prot.contains(PageProtect::WRITEABLE) {
            flags |= PTE_W;
        }
        if prot.contains(PageProtect::EXECUTABLE) {
            flags |= PTE_X;
        }
        if prot.contains(PageProtect::USER) {
            flags |= PTE_U;
        }

        if flags & (PTE_R | PTE_W | PTE_X) == 0 {
            // Invalid leaf entry pattern: this does not mean an inaccessible leaf page,
            // but it is a pointer to the next level table!
            debug_warn!("map_page: {} -> {} has no permissions", vaddr, paddr);
            return Err(ErrorCode::InvalidArg);
        }

        let mut iter = PteIter::new(&self.l0_table, vaddr, len)?;
        while let Some(pte) = iter.next_entry()? {
            if pte.is_valid() {
                return Err(ErrorCode::AlreadyMapped);
            }

            pte.set(paddr, flags);
            paddr = paddr.add(PAGE_SIZE);
            // FIXME: Invalidate TLB
        }
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
        let table = PageTable::new()?;
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
        let old_satp: u64;
        unsafe {
            asm!("csrr {}, satp", out(reg) old_satp);
        }

        // Do nothing if the current CPU is already in the same page table so
        // that we don't flush the TLB needlessly.
        if old_satp == self.satp {
            return;
        }

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
