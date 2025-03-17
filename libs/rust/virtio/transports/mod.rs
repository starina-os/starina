use starina::address::DAddr;
use starina::iobus::IoBus;
use starina::prelude::*;

use super::DeviceType;
use crate::VirtioAttachError;
use crate::virtqueue::VirtQueue;

pub mod mmio;

const VIRTIO_STATUS_ACK: u8 = 1;
const VIRTIO_STATUS_DRIVER: u8 = 2;
const VIRTIO_STATUS_DRIVER_OK: u8 = 4;
const VIRTIO_STATUS_FEAT_OK: u8 = 8;
// const VIRTIO_F_VERSION_1: u64 = 1 << 32;

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct IsrStatus(pub u8);

const QUEUE_INTR: u8 = 1 << 0;
const DEVICE_CONFIG_INTR: u8 = 1 << 1;

impl IsrStatus {
    pub fn queue_intr(&self) -> bool {
        (self.0 & QUEUE_INTR) != 0
    }

    pub fn device_config_intr(&self) -> bool {
        (self.0 & DEVICE_CONFIG_INTR) != 0
    }
}

pub trait VirtioTransport: Send + Sync {
    fn probe(&mut self) -> Option<DeviceType>;
    fn is_modern(&mut self) -> bool;
    fn read_device_config8(&mut self, offset: u16) -> u8;
    fn read_isr_status(&mut self) -> IsrStatus;
    fn ack_interrupt(&mut self, status: IsrStatus);
    fn read_device_status(&mut self) -> u8;
    fn write_device_status(&mut self, value: u8);
    fn read_device_features(&mut self) -> u64;
    fn write_driver_features(&mut self, value: u64);
    fn select_queue(&mut self, index: u16);
    fn queue_max_size(&mut self) -> u16;
    fn set_queue_size(&mut self, queue_size: u16);
    fn notify_queue(&mut self, index: u16);
    fn enable_queue(&mut self);
    fn set_queue_desc_paddr(&mut self, paddr: DAddr);
    fn set_queue_driver_paddr(&mut self, paddr: DAddr);
    fn set_queue_device_paddr(&mut self, paddr: DAddr);
}

impl dyn VirtioTransport {
    fn set_device_status_bit(&mut self, new_bits: u8) {
        let status = self.read_device_status();
        self.write_device_status(status | new_bits);
    }

    pub fn initialize(
        &mut self,
        iobus: &IoBus,
        features: u64,
        num_virtqueues: u16,
    ) -> Result<Vec<Option<VirtQueue>>, VirtioAttachError> {
        // "3.1.1 Driver Requirements: Device Initialization"
        self.write_device_status(0); // Reset the device.
        self.set_device_status_bit(VIRTIO_STATUS_ACK);
        self.set_device_status_bit(VIRTIO_STATUS_DRIVER);
        let device_features = self.read_device_features();
        if (device_features & features) != features {
            warn!(
                "virtio: feature negotiation failure: driver={:x}, device={:x}, unspported={:x}",
                features,
                device_features,
                features & !device_features
            );
            return Err(VirtioAttachError::MissingFeatures);
        }

        self.write_driver_features(features);
        self.set_device_status_bit(VIRTIO_STATUS_FEAT_OK);

        if (self.read_device_status() & VIRTIO_STATUS_FEAT_OK) == 0 {
            return Err(VirtioAttachError::FeatureNegotiationFailure);
        }

        // Initialize virtqueues.
        let mut virtqueues = Vec::new();
        for index in 0..num_virtqueues {
            virtqueues.push(Some(VirtQueue::new(iobus, index, self)));
        }

        self.set_device_status_bit(VIRTIO_STATUS_DRIVER_OK);

        Ok(virtqueues)
    }
}
