use core::fmt;

use crate::address::GPAddr;

pub const VCPU_EXIT_NONE: u8 = 0x0;
pub const VCPU_EXIT_PAGE_FAULT: u8 = 0x1;
pub const VCPU_EXIT_REBOOT: u8 = 0x2;
pub const VCPU_EXIT_IDLE: u8 = 0x3;

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
#[repr(u8)]
pub enum ExitPageFaultKind {
    None = 0,
    Load = 1,
    Store = 2,
    Execute = 3,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ExitPageFault {
    pub gpaddr: GPAddr,
    pub data: [u8; 8],
    pub kind: ExitPageFaultKind,
    pub width: u8,
    pub load_inst: LoadInst,
    pub inst_len: u8,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct LoadInst {
    #[cfg(target_arch = "riscv64")]
    pub rd: u8,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub union ExitInfo {
    empty: (),
    page_fault: ExitPageFault,
}

impl ExitInfo {
    pub fn empty() -> Self {
        Self { empty: () }
    }

    pub fn page_fault(page_fault: ExitPageFault) -> Self {
        Self { page_fault }
    }

    pub fn as_page_fault(&self) -> &ExitPageFault {
        unsafe { &self.page_fault }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct VCpuRunState {
    pub irqs: u32,
    pub exit_reason: u8,
    pub exit_info: ExitInfo,
}

impl fmt::Debug for VCpuRunState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VCpuExitState")
            .field("irqs", &self.irqs)
            .field("exit_reason", &self.exit_reason)
            .finish()
    }
}

#[derive(Debug)]
pub enum VCpuExit<'a> {
    Reboot,
    Idle,
    LoadPageFault { gpaddr: GPAddr, data: &'a mut [u8] },
    StorePageFault { gpaddr: GPAddr, data: &'a mut [u8] },
}

impl VCpuRunState {
    pub fn new() -> Self {
        Self {
            irqs: 0,
            exit_reason: VCPU_EXIT_NONE,
            exit_info: ExitInfo::empty(),
        }
    }

    pub fn as_exit(&mut self) -> VCpuExit<'_> {
        match self.exit_reason {
            VCPU_EXIT_PAGE_FAULT => {
                let page_fault = unsafe { &mut self.exit_info.page_fault };
                match page_fault.kind {
                    ExitPageFaultKind::Load => {
                        VCpuExit::LoadPageFault {
                            gpaddr: page_fault.gpaddr,
                            data: &mut page_fault.data[..page_fault.width as usize],
                        }
                    }
                    ExitPageFaultKind::Store => {
                        VCpuExit::StorePageFault {
                            gpaddr: page_fault.gpaddr,
                            data: &mut page_fault.data[..page_fault.width as usize],
                        }
                    }
                    _ => panic!("unexpected page fault kind: {}", page_fault.kind as u8),
                }
            }
            VCPU_EXIT_REBOOT => VCpuExit::Reboot,
            VCPU_EXIT_IDLE => VCpuExit::Idle,
            _ => panic!("unexpected exit reason: {}", self.exit_reason),
        }
    }
}
