use starina_types::handle::HandleId;
pub use starina_types::vcpu::*;

use crate::error::ErrorCode;
use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::hvspace::HvSpace;
use crate::syscall;

#[derive(Debug)]
pub struct VCpu {
    handle: OwnedHandle,
}

impl VCpu {
    pub fn new(hvspace: &HvSpace, entry: usize) -> Result<Self, ErrorCode> {
        let id = syscall::sys_vcpu_create(hvspace.handle_id(), entry)?;
        Ok(Self {
            handle: OwnedHandle::from_raw(id),
        })
    }

    pub fn run(&self, exit: &mut VCpuExit) -> Result<(), ErrorCode> {
        syscall::sys_vcpu_run(self.handle.id(), exit as *mut _)?;
        Ok(())
    }
}

impl Handleable for VCpu {
    fn handle_id(&self) -> HandleId {
        self.handle.id()
    }
}
