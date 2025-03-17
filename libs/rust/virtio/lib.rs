#![no_std]

pub mod transports;
pub mod virtqueue;

#[derive(Debug, PartialEq, Eq)]
pub enum DeviceType {
    Net,
    Blk,
    Console,
    Unknown(u32),
}

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
