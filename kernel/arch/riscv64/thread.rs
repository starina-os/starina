use ftl_types::handle::HandleRights;
use ftl_types::vmspace::PageProtect;
use ftl_utils::byte_size::ByteSize;

use crate::folio::Folio;
use crate::handle::Handle;
use crate::ref_counted::SharedRef;
use crate::vmspace::VmSpace;

const KERNEL_STACK_SIZE: ByteSize = ByteSize::from_kib(64);

// TODO: static assert to ensure usize == u64

/// Context of a thread.
#[derive(Debug, Default)]
#[repr(C, packed)]
pub struct Context {
    pub sepc: usize,
    pub sstatus: usize,
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s0: usize,
    pub s1: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
}

pub struct Thread {
    pub(super) context: Context,
    pub(super) vmspace: Option<SharedRef<VmSpace>>,
    #[allow(dead_code)]
    stack_folio: Option<Handle<Folio>>,
}

impl Thread {
    pub fn new_idle() -> Thread {
        Thread {
            stack_folio: None,
            vmspace: None,
            context: Default::default(),
        }
    }

    pub fn new_kernel(vmspace: SharedRef<VmSpace>, pc: usize, arg: usize) -> Thread {
        let stack_size = KERNEL_STACK_SIZE.in_bytes();

        let stack_folio = Handle::new(
            SharedRef::new(Folio::alloc(stack_size).unwrap()),
            HandleRights::NONE,
        );
        let stack_vaddr = vmspace
            .map_anywhere(
                stack_size,
                stack_folio.clone(),
                PageProtect::READABLE | PageProtect::WRITABLE,
            )
            .unwrap();

        let mut sstatus: usize;
        unsafe {
            core::arch::asm!("csrr {}, sstatus", out(reg) sstatus);
        }

        let sp = stack_vaddr.as_usize() + stack_size;
        Thread {
            stack_folio: Some(stack_folio),
            vmspace: Some(vmspace),
            context: Context {
                sepc: pc as usize,
                sstatus,
                sp,
                a0: arg,
                ..Default::default()
            },
        }
    }
}
