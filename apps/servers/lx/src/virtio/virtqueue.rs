use starina::address::GPAddr;

use super::device::VirtioDevice;
use crate::guest_memory::GuestMemory;

pub const VIRTQUEUE_NUM_DESCS_MAX: u32 = 256;

const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct VirtqDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

impl VirtqDesc {
    pub fn is_writable(&self) -> bool {
        self.flags & VIRTQ_DESC_F_WRITE != 0
    }

    pub fn has_next(&self) -> bool {
        self.flags & VIRTQ_DESC_F_NEXT != 0
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct VirtqAvail {
    flags: u16,
    index: u16,
    // The rings (an array of descriptor indices) immediately follows here.
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct VirtqUsedElem {
    id: u32,
    len: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct VirtqUsed {
    flags: u16,
    index: u16,
    // The rings (an array of VirtqUsedElem) immediately follows here.
}

struct DescChain {}

pub struct Virtqueue {
    desc_gpaddr: GPAddr,
    device_gpaddr: GPAddr,
    driver_gpaddr: GPAddr,
    num_descs: u32,
}

impl Virtqueue {
    pub fn new() -> Self {
        Self {
            desc_gpaddr: GPAddr::new(0),
            device_gpaddr: GPAddr::new(0),
            driver_gpaddr: GPAddr::new(0),
            num_descs: VIRTQUEUE_NUM_DESCS_MAX,
        }
    }

    pub fn set_queue_size(&mut self, value: u32) {
        debug_assert!(value <= VIRTQUEUE_NUM_DESCS_MAX);

        self.num_descs = value;
    }

    pub fn set_desc_addr(&mut self, value: u32, is_high: bool) {
        set_addr(&mut self.desc_gpaddr, value, is_high);
    }

    pub fn set_device_addr(&mut self, value: u32, is_high: bool) {
        set_addr(&mut self.device_gpaddr, value, is_high);
    }

    pub fn set_driver_addr(&mut self, value: u32, is_high: bool) {
        set_addr(&mut self.driver_gpaddr, value, is_high);
    }

    pub fn queue_notify(&self, memory: &mut GuestMemory, device: &dyn VirtioDevice) {
        while let Some(chain) = self.pop(memory) {
            // TODO: Implement
        }
    }

    fn pop(&self, memory: &mut GuestMemory) -> Option<DescChain> {
        // TODO: Implement
        None
    }
}

fn set_addr(gpaddr: &mut GPAddr, value: u32, is_high: bool) {
    let mut addr = gpaddr.as_usize();
    if is_high {
        addr = (addr & !0xffffffff_usize) | (value as usize);
    } else {
        addr = (addr & 0xffffffff_usize) | ((value as usize) << 32);
    }
    *gpaddr = GPAddr::new(addr);
}
