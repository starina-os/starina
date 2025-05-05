#![no_std]

pub mod autogen;
mod device_tree;
mod guest_memory;
mod linux_loader;
mod virtio;

use device_tree::build_fdt;
use guest_memory::GuestMemory;
use guest_memory::Ram;
use starina::address::GPAddr;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::prelude::*;
use starina::vcpu::VCpu;
use starina::vcpu::VCpuExit;
use starina::vcpu::VCpuExitState;
use virtio::virtio_fs::VirtioFs;

#[derive(Debug)]
pub enum State {}

pub struct App {}

impl EventLoop for App {
    type Env = autogen::Env;
    type State = State;

    fn init(_dispatcher: &dyn Dispatcher<Self::State>, _env: Self::Env) -> Self {
        info!("starting");

        const LINUX_ELF: &[u8] = include_bytes!("../linux/arch/riscv/boot/Image");
        const NUM_CPUS: u32 = 1;
        const GUEST_RAM_SIZE: usize = 64 * 1024 * 1024; // 64MB
        const PLIC_BASE_ADDR: GPAddr = GPAddr::new(0x0a00_0000);
        const VIRTIO_FS_ADDR: GPAddr = GPAddr::new(0x0b00_0000);
        const GUEST_RAM_ADDR: GPAddr = GPAddr::new(0x8000_0000);
        let mut ram = Ram::new(GUEST_RAM_ADDR, GUEST_RAM_SIZE).unwrap();
        let fdt = build_fdt(
            NUM_CPUS,
            GUEST_RAM_ADDR,
            GUEST_RAM_SIZE as u64,
            PLIC_BASE_ADDR,
            &[VIRTIO_FS_ADDR],
        )
        .expect("failed to build device tree");
        let (fdt_slice, fdt_gpaddr) = ram.allocate(fdt.len(), 4096).unwrap();
        fdt_slice[..fdt.len()].copy_from_slice(&fdt);

        // Prepare the guest memory.
        let entry = linux_loader::load_riscv_image(&mut ram, LINUX_ELF).unwrap();

        let mut guest_memory = GuestMemory::new().unwrap();
        guest_memory.add_ram(ram).unwrap();

        let virtio_fs = VirtioFs::new();
        guest_memory
            .add_virtio_mmio(VIRTIO_FS_ADDR, virtio_fs)
            .unwrap();

        // Fill registers that Linux expects:
        //
        // > $a0 to contain the hartid of the current core.
        // > $a1 to contain the address of the devicetree in memory.
        // > https://www.kernel.org/doc/html/next/riscv/boot.html
        let a0 = 0; // hartid
        let a1 = fdt_gpaddr.as_usize(); // fdt address

        let vcpu = VCpu::new(guest_memory.hvspace(), entry.as_usize(), a0, a1).unwrap();
        let mut exit_state = VCpuExitState::new();
        info!("running vcpu");
        loop {
            vcpu.run(&mut exit_state).unwrap();
            match exit_state.as_exit() {
                VCpuExit::LoadPageFault {
                    gpaddr,
                    data,
                    width,
                } => {}
                VCpuExit::StorePageFault {
                    gpaddr,
                    data,
                    width,
                } => {}
            }
        }
        // Self {}
    }
}
