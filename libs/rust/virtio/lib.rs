#![no_std]

pub mod transports;
pub mod virtqueue;

pub type DeviceType = u32;
pub const VIRTIO_DEVICE_TYPE_NET: DeviceType = 1;
pub const VIRTIO_DEVICE_TYPE_BLK: DeviceType = 2;
pub const VIRTIO_DEVICE_TYPE_CONSOLE: DeviceType = 3;

#[derive(Debug)]
pub enum VirtioAttachError {
    UnexpectedDeviceType(DeviceType),
    MissingFeatures,
    MissingPciCommonCfg,
    MissingPciDeviceCfg,
    MissingPciIsrCfg,
    MissingPciNotifyCfg,
    FeatureNegotiationFailure,
    NotSupportedBarType,
}
