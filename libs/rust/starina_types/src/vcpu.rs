use crate::address::GPAddr;

pub const VCPU_EXIT_NONE: u8 = 0x0;
pub const VCPU_EXIT_PAGE_FAULT: u8 = 0x1;

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
    none: (),
    pub page_fault: ExitPageFault,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct VCpuExitState {
    pub reason: u8,
    pub info: ExitInfo,
}

#[derive(Debug)]
pub enum VCpuExit<'a> {
    LoadPageFault { gpaddr: GPAddr, data: &'a mut [u8] },
    StorePageFault { gpaddr: GPAddr, data: &'a mut [u8] },
}

impl VCpuExitState {
    pub fn new() -> Self {
        Self {
            reason: VCPU_EXIT_NONE,
            info: ExitInfo { none: () },
        }
    }

    pub fn as_exit(&mut self) -> VCpuExit<'_> {
        match self.reason {
            VCPU_EXIT_PAGE_FAULT => {
                let page_fault = unsafe { &mut self.info.page_fault };
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

            _ => panic!("unexpected exit reason: {}", self.reason),
        }
    }
}
