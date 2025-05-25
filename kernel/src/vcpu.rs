use core::fmt;

use starina::error::ErrorCode;
use starina::poll::Readiness;
use starina_types::vcpu::VCpuRunState;

use crate::arch;
use crate::handle::Handleable;
use crate::hvspace::HvSpace;
use crate::isolation::Isolation;
use crate::isolation::IsolationSliceMut;
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
    pub fn new(
        hvspace: SharedRef<HvSpace>,
        entry: usize,
        arg0: usize,
        arg1: usize,
    ) -> Result<VCpu, ErrorCode> {
        let arch = arch::VCpu::new(&hvspace, entry, arg0, arg1)?;
        Ok(VCpu { hvspace, arch })
    }

    // FIXME:
    pub unsafe fn arch_vcpu_ptr(&self) -> *mut arch::VCpu {
        &raw const self.arch as *mut _
    }

    pub fn apply_state(
        &self,
        isolation: &dyn Isolation,
        run_state: IsolationSliceMut,
    ) -> Result<(), ErrorCode> {
        self.arch.apply_state(isolation, run_state)
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
