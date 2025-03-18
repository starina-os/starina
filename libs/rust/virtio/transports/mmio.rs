use starina::address::DAddr;
use starina::folio::MmioFolio;
use starina::prelude::*;
use starina_driver_sdk::mmio::LittleEndian;
use starina_driver_sdk::mmio::MmioReg;
use starina_driver_sdk::mmio::ReadOnly;
use starina_driver_sdk::mmio::ReadWrite;
use starina_driver_sdk::mmio::WriteOnly;

use super::VirtioTransport;
use crate::DeviceType;
use crate::transports::IsrStatus;

// "All register values are organized as Little Endian."
// (4.2.2 MMIO Device Register Layout).
static MAGIC_VALUE_REG: MmioReg<LittleEndian, ReadOnly, u32> = MmioReg::new(0x00);
static DEVICE_VERSION_REG: MmioReg<LittleEndian, ReadOnly, u32> = MmioReg::new(0x04);
static DEVICE_ID_REG: MmioReg<LittleEndian, ReadOnly, u32> = MmioReg::new(0x08);
static DEVICE_FEATURES_REG: MmioReg<LittleEndian, ReadOnly, u32> = MmioReg::new(0x10);
static DEVICE_FEATURES_SEL_REG: MmioReg<LittleEndian, WriteOnly, u32> = MmioReg::new(0x14);
static DRIVER_FEATURES_REG: MmioReg<LittleEndian, WriteOnly, u32> = MmioReg::new(0x20);
static DRIVER_FEATURES_SEL_REG: MmioReg<LittleEndian, WriteOnly, u32> = MmioReg::new(0x24);
static QUEUE_SEL_REG: MmioReg<LittleEndian, WriteOnly, u32> = MmioReg::new(0x30);
static QUEUE_NUM_MAX_REG: MmioReg<LittleEndian, ReadOnly, u32> = MmioReg::new(0x34);
static QUEUE_NUM_REG: MmioReg<LittleEndian, WriteOnly, u32> = MmioReg::new(0x38);
static QUEUE_READY_REG: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0x44);
static QUEUE_NOTIFY_REG: MmioReg<LittleEndian, WriteOnly, u32> = MmioReg::new(0x50);
static INTERRUPT_STATUS_REG: MmioReg<LittleEndian, ReadOnly, u32> = MmioReg::new(0x60);
static INTERRUPT_ACK_REG: MmioReg<LittleEndian, WriteOnly, u32> = MmioReg::new(0x64);
static DEVICE_STATUS_REG: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0x70);
static QUEUE_DESC_LOW_REG: MmioReg<LittleEndian, WriteOnly, u32> = MmioReg::new(0x80);
static QUEUE_DESC_HIGH_REG: MmioReg<LittleEndian, WriteOnly, u32> = MmioReg::new(0x84);
static QUEUE_DRIVER_LOW_REG: MmioReg<LittleEndian, WriteOnly, u32> = MmioReg::new(0x90);
static QUEUE_DRIVER_HIGH_REG: MmioReg<LittleEndian, WriteOnly, u32> = MmioReg::new(0x94);
static QUEUE_DEVICE_LOW_REG: MmioReg<LittleEndian, WriteOnly, u32> = MmioReg::new(0xa0);
static QUEUE_DEVICE_HIGH_REG: MmioReg<LittleEndian, WriteOnly, u32> = MmioReg::new(0xa4);
static CONFIG_REG_BASE: MmioReg<LittleEndian, ReadWrite, u8> = MmioReg::new(0x100);

pub struct VirtioMmio {
    mmio: MmioFolio,
}

impl VirtioMmio {
    pub fn new(mmio: MmioFolio) -> VirtioMmio {
        VirtioMmio { mmio }
    }
}

impl VirtioTransport for VirtioMmio {
    fn probe(&mut self) -> Option<DeviceType> {
        // Check if the device is present by checking t he magic value
        // ("virt" in little-endian).
        if MAGIC_VALUE_REG.read(&mut self.mmio) != 0x74726976 {
            return None;
        }

        let version = DEVICE_VERSION_REG.read(&mut self.mmio);
        if version != 2 {
            warn!("virtio-mmio: unsupported device version: {}", version);
            return None;
        }

        let device_type = DEVICE_ID_REG.read(&mut self.mmio);
        match device_type {
            1 => Some(DeviceType::Net),
            2 => Some(DeviceType::Blk),
            3 => Some(DeviceType::Console),
            _ => Some(DeviceType::Unknown(device_type)),
        }
    }

    fn is_modern(&mut self) -> bool {
        true
    }

    fn read_device_config8(&mut self, offset: u16) -> u8 {
        CONFIG_REG_BASE.read_with_offset(&mut self.mmio, offset as usize)
    }

    fn read_isr_status(&mut self) -> IsrStatus {
        IsrStatus(INTERRUPT_STATUS_REG.read(&mut self.mmio) as u8)
    }

    fn ack_interrupt(&mut self, status: IsrStatus) {
        INTERRUPT_ACK_REG.write(&mut self.mmio, status.0 as u32);
    }

    fn read_device_status(&mut self) -> u8 {
        DEVICE_STATUS_REG.read(&mut self.mmio) as u8
    }

    fn write_device_status(&mut self, value: u8) {
        DEVICE_STATUS_REG.write(&mut self.mmio, value as u32);
    }

    fn read_device_features(&mut self) -> u64 {
        DEVICE_FEATURES_SEL_REG.write(&mut self.mmio, 0);
        let low = DEVICE_FEATURES_REG.read(&mut self.mmio);
        DEVICE_FEATURES_SEL_REG.write(&mut self.mmio, 1);
        let high = DEVICE_FEATURES_REG.read(&mut self.mmio);
        ((high as u64) << 32) | (low as u64)
    }

    fn write_driver_features(&mut self, value: u64) {
        DRIVER_FEATURES_SEL_REG.write(&mut self.mmio, 0);
        DRIVER_FEATURES_REG.write(&mut self.mmio, (value & 0xffff_ffff) as u32);
        DRIVER_FEATURES_SEL_REG.write(&mut self.mmio, 1);
        DRIVER_FEATURES_REG.write(&mut self.mmio, (value >> 32) as u32);
    }

    fn select_queue(&mut self, index: u16) {
        QUEUE_SEL_REG.write(&mut self.mmio, index as u32);
    }

    fn queue_max_size(&mut self) -> u16 {
        QUEUE_NUM_MAX_REG.read(&mut self.mmio) as u16
    }

    fn set_queue_size(&mut self, queue_size: u16) {
        QUEUE_NUM_REG.write(&mut self.mmio, queue_size as u32);
    }

    fn notify_queue(&mut self, index: u16) {
        QUEUE_NOTIFY_REG.write(&mut self.mmio, index as u32);
    }

    fn enable_queue(&mut self) {
        QUEUE_READY_REG.write(&mut self.mmio, 1);
    }

    fn set_queue_desc_paddr(&mut self, daddr: DAddr) {
        QUEUE_DESC_LOW_REG.write(&mut self.mmio, (daddr.as_usize() & 0xffff_ffff) as u32);
        QUEUE_DESC_HIGH_REG.write(&mut self.mmio, (daddr.as_usize() >> 32) as u32);
    }

    fn set_queue_driver_paddr(&mut self, daddr: DAddr) {
        info!("daddr.as_usize() = {:x}", daddr.as_usize());
        QUEUE_DRIVER_LOW_REG.write(&mut self.mmio, (daddr.as_usize() & 0xffff_ffff) as u32);
        QUEUE_DRIVER_HIGH_REG.write(&mut self.mmio, (daddr.as_usize() >> 32) as u32);
    }

    fn set_queue_device_paddr(&mut self, daddr: DAddr) {
        QUEUE_DEVICE_LOW_REG.write(&mut self.mmio, (daddr.as_usize() & 0xffff_ffff) as u32);
        QUEUE_DEVICE_HIGH_REG.write(&mut self.mmio, (daddr.as_usize() >> 32) as u32);
    }
}
