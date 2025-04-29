#![no_std]

pub mod autogen;
mod device_tree;
mod guest_memory;
mod linux_loader;

use device_tree::build_fdt;
use guest_memory::GuestMemory;
use guest_memory::Ram;
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

        const LINUX_ELF: &[u8] = include_bytes!("../linux.bin");
        const GUEST_RAM_SIZE: usize = 64 * 1024 * 1024; // 64MB
        const GUEST_RAM_ADDR: GPAddr = GPAddr::new(0x80000000);

        let mut ram = Ram::new(GUEST_RAM_ADDR, GUEST_RAM_SIZE).unwrap();
        let fdt = build_fdt().expect("failed to build device tree");
        let (fdt_slice, _) = ram.allocate(fdt.len(), 4096).unwrap();
        fdt_slice[..fdt.len()].copy_from_slice(&fdt);

        // Prepare the guest memory.
        let entry = linux_loader::load_riscv_image(&mut ram, LINUX_ELF).unwrap();

        let mut guest_memory = GuestMemory::new().unwrap();
        guest_memory.add_ram(ram).unwrap();

        // Create a vCPU and run it.
        let vcpu = VCpu::new(guest_memory.hvspace(), entry.as_usize()).unwrap();
        let mut exit = VCpuExit::new();
        loop {
            trace!("entering vcpu.run");
            vcpu.run(&mut exit).unwrap();
        }
        // Self {}
    }
}
