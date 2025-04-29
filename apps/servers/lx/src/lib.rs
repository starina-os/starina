#![no_std]

pub mod autogen;
mod device_tree;
mod linux_loader;

use device_tree::build_fdt;
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

        // Prepare the guest memory.
        let hvspace = HvSpace::new().unwrap();
        let entry = linux_loader::load_riscv_image(&mut hvspace, LINUX_ELF).unwrap();

        let fdt = build_fdt().expect("failed to build device tree");

        // Create a vCPU and run it.
        let vcpu = VCpu::new(&hvspace, entry).unwrap();
        let mut exit = VCpuExit::new();
        loop {
            trace!("entering vcpu.run");
            vcpu.run(&mut exit).unwrap();
        }
        // Self {}
    }
}
