use core::cmp::min;
use core::slice;

use starina::prelude::*;

use super::device::VirtioDevice;

#[repr(C)]
struct VirtioConfig {
    tag: [u8; 36],
    num_request_queues: u32,
    notify_buf_size: u32,
}

pub struct VirtioFs {}

impl VirtioFs {
    pub fn new() -> Self {
        Self {}
    }
}

impl VirtioDevice for VirtioFs {
    fn num_queues(&self) -> u32 {
        3
    }

    fn device_features(&self) -> u64 {
        0
    }

    fn driver_features(&self) -> u64 {
        0
    }

    fn device_id(&self) -> u32 {
        26
    }

    fn vendor_id(&self) -> u32 {
        0
    }

    fn config_read(&self, offset: u64, buf: &mut [u8]) {
        let config = VirtioConfig {
            tag: b"rootfs\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            num_request_queues: 1_u32.to_le(),
            notify_buf_size: 0_u32.to_le(),
        };
        let config_size = size_of::<VirtioConfig>();

        let config_bytes: &[u8] =
            unsafe { slice::from_raw_parts(&config as *const _ as *const u8, config_size) };

        let offset = offset as usize;
        if offset >= config_size {
            debug_warn!("virtio-fs: config read: offset={:x} out of range", offset);
            return;
        }

        let copy_len = min(buf.len(), config_size.saturating_sub(offset));
        buf[..copy_len].copy_from_slice(&config_bytes[offset..offset + copy_len]);
    }
}
