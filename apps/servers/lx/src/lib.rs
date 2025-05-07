#![no_std]

pub mod autogen;
mod guest_memory;
mod linux_loader;
mod mmio;
mod riscv;
mod virtio;

use guest_memory::GuestMemory;
use mmio::Bus;
use riscv::device_tree::build_fdt;
use riscv::plic::Plic;
use riscv::plic::plic_mmio_size;
use starina::address::GPAddr;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::prelude::*;
use starina::vcpu::VCpu;
use starina::vcpu::VCpuExit;
use starina::vcpu::VCpuExitState;
use virtio::device::VIRTIO_MMIO_SIZE;
use virtio::device::VirtioMmio;
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
        const PLIC_MMIO_SIZE: usize = plic_mmio_size(NUM_CPUS);
        debug_assert!(PLIC_BASE_ADDR.as_usize() + PLIC_MMIO_SIZE <= VIRTIO_FS_ADDR.as_usize());
        const VIRTIO_FS_ADDR: GPAddr = GPAddr::new(0x0b00_0000);
        const GUEST_RAM_ADDR: GPAddr = GPAddr::new(0x8000_0000);

        let mut memory = GuestMemory::new(GUEST_RAM_ADDR, GUEST_RAM_SIZE).unwrap();

        let fdt = build_fdt(
            NUM_CPUS,
            GUEST_RAM_ADDR,
            GUEST_RAM_SIZE as u64,
            PLIC_BASE_ADDR,
            PLIC_MMIO_SIZE,
            &[VIRTIO_FS_ADDR],
        )
        .expect("failed to build device tree");
        let (fdt_slice, fdt_gpaddr) = memory.allocate(fdt.len(), 4096).unwrap();
        fdt_slice[..fdt.len()].copy_from_slice(&fdt);

        // Prepare the guest memory.
        let entry = linux_loader::load_riscv_image(&mut memory, LINUX_ELF).unwrap();

        let mut bus = Bus::new();
        let virtio_fs = VirtioFs::new();
        let virtio_mmio_fs = VirtioMmio::new(virtio_fs).unwrap();
        bus.add_device(VIRTIO_FS_ADDR, VIRTIO_MMIO_SIZE, virtio_mmio_fs);

        let plic = Plic::new();
        bus.add_device(PLIC_BASE_ADDR, PLIC_MMIO_SIZE, plic);

        // Fill registers that Linux expects:
        //
        // > $a0 to contain the hartid of the current core.
        // > $a1 to contain the address of the devicetree in memory.
        // > https://www.kernel.org/doc/html/next/riscv/boot.html
        let a0 = 0; // hartid
        let a1 = fdt_gpaddr.as_usize(); // fdt address

        let vcpu = VCpu::new(memory.hvspace(), entry.as_usize(), a0, a1).unwrap();
        let mut exit_state = VCpuExitState::new();
        loop {
            vcpu.run(&mut exit_state).unwrap();
            match exit_state.as_exit() {
                VCpuExit::LoadPageFault { gpaddr, data } => {
                    bus.read(&mut memory, gpaddr, data).unwrap();
                }
                VCpuExit::StorePageFault { gpaddr, data } => {
                    bus.write(&mut memory, gpaddr, data).unwrap();
                }
            }
        }
        // Self {}
    }
}
