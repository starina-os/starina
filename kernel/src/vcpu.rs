use core::fmt;

use starina::error::ErrorCode;
use starina::poll::Readiness;

use crate::arch;
use crate::handle::Handleable;
use crate::hvspace::HvSpace;
use crate::poll::Listener;
use crate::poll::Poll;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;
use crate::thread::Thread;

struct Mutable {
    in_use: bool,
}

pub struct VCpu {
    hvspace: SharedRef<HvSpace>,
    arch: arch::VCpu,
}

impl VCpu {
    pub fn new(hvspace: SharedRef<HvSpace>, entry: usize) -> Result<VCpu, ErrorCode> {
        let arch = arch::VCpu::new(&hvspace, entry)?;
        Ok(VCpu { hvspace, arch })
    }

    // FIXME:
    pub unsafe fn arch_vcpu_ptr(&self) -> *mut arch::VCpu {
        &raw const self.arch as *mut _
    }

    pub fn run(&self, current: &SharedRef<Thread>) -> ! {
        todo!()
    }
}

impl fmt::Debug for VCpu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VCpu").finish()
    }
}

impl Handleable for VCpu {
    fn close(&self) {
        // Do nothing
    }

    fn add_listener(&self, _listener: Listener) -> Result<(), ErrorCode> {
        debug_warn!("unsupported method at {}:{}", file!(), line!());
        Err(ErrorCode::NotSupported)
    }

    fn remove_listener(&self, _poll: &Poll) -> Result<(), ErrorCode> {
        debug_warn!("unsupported method at {}:{}", file!(), line!());
        Err(ErrorCode::NotSupported)
    }

    fn readiness(&self) -> Result<Readiness, ErrorCode> {
        debug_warn!("unsupported method at {}:{}", file!(), line!());
        Err(ErrorCode::NotSupported)
    }
}
