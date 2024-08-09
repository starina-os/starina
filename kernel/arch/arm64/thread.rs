use core::arch::asm;
use core::mem::offset_of;

use ftl_types::handle::HandleRights;
use ftl_types::vmspace::PageProtect;

use super::cpuvar::CpuVar;
use crate::folio::Folio;
use crate::handle::Handle;
use crate::ref_counted::SharedRef;
use crate::scheduler::GLOBAL_SCHEDULER;
use crate::vmspace::KERNEL_VMSPACE;

/// The entry point of kernel threads.
#[no_mangle]
#[naked]
extern "C" fn kernel_entry() -> ! {
    unsafe {
        asm!(
            r#"
                mov lr, x19   // The desired entry point for the thread.
                mov x0, x20    // The argument for the thread.
                ret
            "#,
            options(noreturn)
        )
    }
}

fn switch_to_next() {
    GLOBAL_SCHEDULER.yield_cpu();
}

/// # Why `#[naked]`?
///
/// - To get the correct return address from ra. #[naked] prevents inlining.
/// - To eliminate the needless prologue.
#[naked]
#[no_mangle]
pub extern "C" fn yield_cpu() {
    unsafe {
        asm!(
            r#"
                mrs x0, tpidr_el0
                ldr x0, [x0, #{context_offset}]

                stp x19, x20, [x0, #{x19_offset}]
                stp x21, x22, [x0, #{x21_offset}]
                stp x23, x24, [x0, #{x23_offset}]
                stp x25, x26, [x0, #{x25_offset}]
                stp x27, x28, [x0, #{x27_offset}]
                stp x29, x30, [x0, #{x29_offset}]

                mov x1, sp
                str x1, [x0, #{sp_offset}]

                b {switch_to_next}
            "#,
            context_offset = const offset_of!(CpuVar, context),
            x19_offset = const offset_of!(Context, x19),
            x21_offset = const offset_of!(Context, x21),
            x23_offset = const offset_of!(Context, x23),
            x25_offset = const offset_of!(Context, x25),
            x27_offset = const offset_of!(Context, x27),
            x29_offset = const offset_of!(Context, fp /* aka x29 */),
            sp_offset = const offset_of!(Context, sp),
            switch_to_next = sym switch_to_next,
            options(noreturn)
        )
    }
}

/// Resumes a thread.
#[no_mangle]
fn resume(next: *mut Context) -> ! {
    unsafe {
        asm!(
            r#"
                // Update CpuVar.arch.context
                mrs x1, tpidr_el0
                str x0, [x1, #{context_offset}]

                ldp x19, x20, [x0, #{x19_offset}]
                ldp x21, x22, [x0, #{x21_offset}]
                ldp x23, x24, [x0, #{x23_offset}]
                ldp x25, x26, [x0, #{x25_offset}]
                ldp x27, x28, [x0, #{x27_offset}]
                ldp x29, x30, [x0, #{x29_offset}]

                ldr x1, [x0, #{sp_offset}]
                mov sp, x1

                ret
            "#,
            in ("x0") next as usize,
            context_offset = const offset_of!(CpuVar, context),
            x19_offset = const offset_of!(Context, x19),
            x21_offset = const offset_of!(Context, x21),
            x23_offset = const offset_of!(Context, x23),
            x25_offset = const offset_of!(Context, x25),
            x27_offset = const offset_of!(Context, x27),
            x29_offset = const offset_of!(Context, fp /* aka x29 */),
            sp_offset = const offset_of!(Context, sp),
            options(noreturn)
        )
    }
}

/// Context of a thread.
///
/// Only callee-saved registers are stored because caller-saved registers are
/// already saved on the stack.
#[derive(Debug, Default)]
#[repr(C)]
pub struct Context {
    x19: usize,
    x20: usize,
    x21: usize,
    x22: usize,
    x23: usize,
    x24: usize,
    x25: usize,
    x26: usize,
    x27: usize,
    x28: usize,
    fp: usize, // aka x29
    lr: usize, // aka x30
    sp: usize,
}

pub struct Thread {
    pub(super) context: Context,
    #[allow(dead_code)]
    stack_folio: Option<Handle<Folio>>,
}

impl Thread {
    pub fn new_idle() -> Thread {
        Thread {
            context: Default::default(),
            stack_folio: None,
        }
    }

    pub fn new_kernel(pc: usize, arg: usize) -> Thread {
        let stack_size = 64 * 1024;
        let stack_folio = Handle::new(
            SharedRef::new(Folio::alloc(stack_size).unwrap()),
            HandleRights::NONE,
        );
        let stack_vaddr = KERNEL_VMSPACE
            .map(
                stack_size,
                stack_folio.clone(),
                PageProtect::READABLE | PageProtect::WRITABLE,
            )
            .unwrap();

        let sp = stack_vaddr.as_usize() + stack_size;
        Thread {
            context: Context {
                lr: kernel_entry as usize,
                sp,
                x19: pc,
                x20: arg,
                ..Default::default()
            },
            stack_folio: Some(stack_folio),
        }
    }

    pub fn resume(&self) -> ! {
        resume(&self.context as *const _ as *mut _);
    }
}
