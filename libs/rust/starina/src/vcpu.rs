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
    exit: VCpuExitState,
}

impl VCpu {
    pub fn new(hvspace: &HvSpace, entry: usize, a0: usize, a1: usize) -> Result<Self, ErrorCode> {
        let id = syscall::sys_vcpu_create(hvspace.handle_id(), entry, a0, a1)?;
        Ok(Self {
            handle: OwnedHandle::from_raw(id),
            exit: VCpuExitState::new(),
        })
    }

    pub fn run(&mut self) -> Result<VCpuExit<'_>, ErrorCode> {
        syscall::sys_vcpu_run(self.handle.id(), &mut self.exit)?;
        Ok(self.exit.as_exit())
    }

    pub fn inject_irqs(&mut self, irqs: u32) {
        self.exit.irqs |= irqs;
    }
}

impl Handleable for VCpu {
    fn handle_id(&self) -> HandleId {
        self.handle.id()
    }
}
