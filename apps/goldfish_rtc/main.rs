//! A Goldfish RTC device driver.
//!
//! <https://android.googlesource.com/platform/external/qemu/+/refs/heads/emu-2.0-release/docs/GOLDFISH-VIRTUAL-HARDWARE.TXT>
#![no_std]
#![no_main]

use starina_api::environ::Environ;
use starina_api::folio::MappedFolio;
use starina_api::prelude::*;
use starina_api::types::address::PAddr;
use starina_driver_utils::mmio::LittleEndian;
use starina_driver_utils::mmio::MmioReg;
use starina_driver_utils::mmio::ReadOnly;

/// <https://github.com/qemu/qemu/blob/01dc65a3bc262ab1bec8fe89775e9bbfa627becb/hw/riscv/virt.c#L74>
const MMIO_BASE: PAddr = PAddr::new(0x101000);

const MMIO_SIZE: usize = 4096;
static TIME_LOW_REG: MmioReg<LittleEndian, ReadOnly, u32> = MmioReg::new(0x00);
static TIME_HIGH_REG: MmioReg<LittleEndian, ReadOnly, u32> = MmioReg::new(0x04);

#[no_mangle]
pub fn main(_env: Environ) {
    let mut folio = MappedFolio::create_pinned(MMIO_BASE, MMIO_SIZE).unwrap();
    let low: u32 = TIME_LOW_REG.read(&mut folio);
    let high: u32 = TIME_HIGH_REG.read(&mut folio);
    let now: u64 = (high as u64) << 32 | (low as u64);

    let now_i64: i64 = now.try_into().unwrap();
    let datetime = chrono::DateTime::from_timestamp_nanos(now_i64);
    info!("now: {datetime}");
}
