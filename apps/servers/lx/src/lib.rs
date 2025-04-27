#![no_std]

pub mod autogen;

use starina::address::GPAddr;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::folio::Folio;
use starina::hvspace::HvSpace;
use starina::prelude::*;
use starina::vcpu::VCpu;
use starina::vcpu::VCpuExit;
use starina::vmspace::PageProtect;
use starina::vmspace::VmSpace;

#[derive(Debug)]
pub enum State {}

pub struct App {}

impl EventLoop for App {
    type Env = autogen::Env;
    type State = State;

    fn init(_dispatcher: &dyn Dispatcher<Self::State>, _env: Self::Env) -> Self {
        info!("starting");

        const GUEST_MEMORY_SIZE: usize = 0x1000;
        const GUEST_ENTRY: usize = 0x8000_a000;

        let folio = Folio::alloc(4096).unwrap();
        let vaddr = VmSpace::map_anywhere_current(
            &folio,
            GUEST_MEMORY_SIZE,
            PageProtect::READABLE | PageProtect::WRITEABLE,
        )
        .unwrap();

        let guest_memory: &mut [u8] =
            unsafe { core::slice::from_raw_parts_mut(vaddr.as_mut_ptr(), GUEST_MEMORY_SIZE) };

        const BOOT_CODE: &[u8] = include_bytes!("../../../../guest.bin");

        // Copy the boot code to the guest memory.
        unsafe {
            core::ptr::copy_nonoverlapping(
                BOOT_CODE.as_ptr(),
                guest_memory.as_mut_ptr(),
                BOOT_CODE.len(),
            );
        };

        let hvspace = HvSpace::new().unwrap();
        hvspace
            .map(
                GPAddr::new(GUEST_ENTRY),
                &folio,
                GUEST_MEMORY_SIZE,
                PageProtect::READABLE | PageProtect::WRITEABLE | PageProtect::EXECUTABLE,
            )
            .unwrap();

        let vcpu = VCpu::new(&hvspace, GUEST_ENTRY).unwrap();
        let mut exit = VCpuExit::new();
        loop {
            trace!("entering vcpu.run");
            vcpu.run(&mut exit).unwrap();
        }
        // Self {}
    }
}
