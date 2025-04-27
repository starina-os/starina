pub const VCPU_EXIT_PAGE_FAULT: u8 = 0x1;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ExitPageFault {
    pub data: [u8; 8],
}

#[derive(Clone, Copy)]
#[repr(C)]
pub union ExitInfo {
    pub page_fault: ExitPageFault,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct VCpuExit {
    pub reason: u8,
    pub info: ExitInfo,
}

impl VCpuExit {
    pub fn new() -> Self {
        Self {
            reason: 0,
            info: ExitInfo {
                page_fault: ExitPageFault { data: [0; 8] },
            },
        }
    }
}
