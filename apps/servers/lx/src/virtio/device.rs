use starina::address::GPAddr;
use starina::error::ErrorCode;
use starina::prelude::*;
use starina::sync::Mutex;

use crate::guest_memory::MmioDevice;
use crate::guest_memory::MmioError;

/// The host-side (device-side) of a virtio device.
///
/// Guest OS interacts with this device through their virtio
/// device drivers.
pub trait VirtioDevice {
    fn num_queues(&self) -> u32;
    fn device_features(&self) -> u64;
    fn driver_features(&self) -> u64;
    fn device_id(&self) -> u32;
    fn vendor_id(&self) -> u32;
}

pub const VIRTIO_MMIO_SIZE: usize = 4096;

// Virtio MMIO registers.
// <https://docs.oasis-open.org/virtio/virtio/v1.3/csd01/virtio-v1.3-csd01.html#:~:text=42%3E%3B%C2%A0%0A%7D-,4.2.2%20MMIO%20Device%20Register%20Layout,-MMIO%20virtio%20devices>
const REG_MAGIC: u64 = 0x00;
const REG_VERSION: u64 = 0x04;
const REG_DEVICE_ID: u64 = 0x08;
const REG_VENDOR_ID: u64 = 0x0c;
const REG_DEVICE_FEATURES: u64 = 0x10;
const REG_DEVICE_FEATURES_SEL: u64 = 0x14;
const REG_DRIVER_FEATURES: u64 = 0x20;
const REG_DRIVER_FEATURES_SEL: u64 = 0x24;
const REG_QUEUE_SELECT: u64 = 0x30;
const REG_QUEUE_NUM_MAX: u64 = 0x34;
const REG_QUEUE_NUM: u64 = 0x38;
const REG_QUEUE_READY: u64 = 0x44;
const REG_QUEUE_NOTIFY: u64 = 0x50;
const REG_INTERRUPT_STATUS: u64 = 0x60;
const REG_INTERRUPT_ACK: u64 = 0x64;
const REG_DEVICE_STATUS: u64 = 0x70;
const REG_QUEUE_DESC_LOW: u64 = 0x80;
const REG_QUEUE_DESC_HIGH: u64 = 0x84;
const REG_QUEUE_DRIVER_LOW: u64 = 0x90;
const REG_QUEUE_DRIVER_HIGH: u64 = 0x94;
const REG_QUEUE_DEVICE_LOW: u64 = 0xa0;
const REG_QUEUE_DEVICE_HIGH: u64 = 0xa4;

const VIRTIO_F_VERSION_1: u64 = 1 << 32;

#[derive(Debug)]
pub enum Error {
    AllocFolio(ErrorCode),
    VmSpaceMap(ErrorCode),
}

struct Mutable {
    device_features_select: u32,
    device_status: u32,
    queue_select: u32,
    num_queues: u32,
}

/// Virtio device over memory-mapped I/O.
pub struct VirtioMmio {
    device: Box<dyn VirtioDevice>,
    mutable: Mutex<Mutable>,
}

impl VirtioMmio {
    pub fn new<D: VirtioDevice + 'static>(device: D) -> Result<Self, Error> {
        let num_queues = device.num_queues();
        Ok(Self {
            device: Box::new(device),
            mutable: Mutex::new(Mutable {
                device_features_select: 0,
                device_status: 0,
                queue_select: 0,
                num_queues,
            }),
        })
    }
}

impl MmioDevice for VirtioMmio {
    fn read(&self, offset: u64, dst: &mut [u8]) -> Result<(), MmioError> {
        trace!(
            "virtio mmio read: offset={:x}, width={:x}",
            offset,
            dst.len()
        );

        let width: usize = dst.len();
        if width != 4 {
            panic!(
                "unexpected virtio-mmio read: offset={:x}, width={}",
                offset, width
            );
        }

        let mutable = self.mutable.lock();
        let value = match offset {
            REG_MAGIC => 0x74726976,
            REG_VERSION => 2,
            REG_DEVICE_ID => self.device.device_id(),
            REG_VENDOR_ID => self.device.vendor_id(),
            REG_DEVICE_FEATURES => {
                let features = self.device.device_features() | VIRTIO_F_VERSION_1;
                if mutable.device_features_select == 0 {
                    (features & 0xffffffff) as u32
                } else {
                    (features >> 32) as u32
                }
            }
            REG_DEVICE_FEATURES_SEL => mutable.device_features_select,
            REG_DEVICE_STATUS => mutable.device_status,
            _ => {
                panic!(
                    "unexpected virtio-mmio read: offset={:x}, width={}",
                    offset, width
                );
            }
        };

        dst.copy_from_slice(&value.to_ne_bytes());
        Ok(())
    }

    fn write(&self, offset: u64, src: &[u8]) -> Result<(), MmioError> {
        trace!(
            "virtio mmio write: offset={:x}, width={:x}",
            offset,
            src.len()
        );

        let width = src.len();
        if width != 4 {
            panic!(
                "unexpected virtio-mmio write: offset={:x}, width={}",
                offset, width
            );
        }

        let value = u32::from_ne_bytes(src.try_into().unwrap());
        let mut mutable = self.mutable.lock();
        match offset {
            REG_DEVICE_FEATURES_SEL => {
                mutable.device_features_select = value;
            }
            REG_DEVICE_STATUS => {
                mutable.device_status = value;
            }
            REG_QUEUE_SELECT => {
                mutable.queue_select = value;
            }
            _ => {
                panic!(
                    "unexpected virtio-mmio write: offset={:x}, width={}",
                    offset, width
                );
            }
        }

        Ok(())
    }
}
