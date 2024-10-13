use crate::refcount::SharedRef;
use crate::vmspace::VmSpace;

/// Context of a thread.
#[derive(Debug, Default)]
#[repr(C, packed)]
pub struct Context {
    pub rip: usize,
    pub rax: usize,
    pub rbx: usize,
    pub rcx: usize,
    pub rdx: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub rbp: usize,
    pub rsp: usize,
    pub r8: usize,
    pub r9: usize,
    pub r10: usize,
    pub r11: usize,
    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,
}

pub struct Thread {
    pub(super) context: Context,
}

impl Thread {
    pub fn new_idle() -> Thread {
        Thread {
            context: Default::default(),
        }
    }

    pub fn new_kernel(pc: usize, sp: usize, arg: usize) -> Thread {
        Thread {
            context: Context {
                rip: pc,
                rsp: sp,
                rdi: arg,
                ..Default::default()
            },
        }
    }
}
