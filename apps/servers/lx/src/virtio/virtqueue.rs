use core::mem::offset_of;

use starina::address::GPAddr;
use starina::collections::VecDeque;
use starina::prelude::*;

use super::device::VirtioDevice;
use crate::guest_memory::GuestMemory;
use crate::guest_memory::{self};

pub const VIRTQUEUE_NUM_DESCS_MAX: u32 = 256;

const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;

/// Used Buffer Notification.
const VIRQ_IRQSTATUS_QUEUE: u32 = 1 << 0;

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct VirtqDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

impl VirtqDesc {
    pub fn gpaddr(&self) -> GPAddr {
        GPAddr::new(self.addr as usize)
    }

    pub fn is_write_only(&self) -> bool {
        self.flags & VIRTQ_DESC_F_WRITE != 0
    }

    pub fn is_read_only(&self) -> bool {
        !self.is_write_only()
    }

    pub fn has_next(&self) -> bool {
        self.flags & VIRTQ_DESC_F_NEXT != 0
    }
}

type DescIndex = u16;

pub struct DescChain {
    head: DescIndex,
}

impl DescChain {
    pub fn reader_writer<'a>(
        &'a self,
        vq: &mut Virtqueue,
        memory: &'a GuestMemory,
    ) -> Result<(DescChainReader<'a>, DescChainWriter<'a>), guest_memory::Error> {
        let mut readable_descs = VecDeque::new();
        let mut writable_descs = VecDeque::new();
        let mut desc_index = self.head;
        loop {
            let desc_gpaddr = vq
                .desc_gpaddr
                .checked_add(desc_index as usize * size_of::<VirtqDesc>())
                .unwrap();

            let desc = memory.read::<VirtqDesc>(desc_gpaddr)?;

            if !desc.has_next() {
                break;
            }

            if desc.is_read_only() {
                readable_descs.push_back(desc);
            } else {
                writable_descs.push_back(desc);
            }

            desc_index = desc.next;
        }

        let reader = DescChainReader {
            head: &self,
            memory,
            descs: readable_descs,
        };

        let writer = DescChainWriter {
            head: &self,
            memory,
            descs: writable_descs,
        };

        Ok((reader, writer))
    }
}

pub struct DescChainWriter<'a> {
    head: &'a DescChain,
    memory: &'a GuestMemory,
    descs: VecDeque<VirtqDesc>,
}

impl<'a> DescChainWriter<'a> {
    pub fn write<T: Copy>(&mut self, value: T) -> Result<(), guest_memory::Error> {
        // TODO: This assumes the guest provides a descriptor per one write in VMM.
        let desc = match self.descs.pop_front() {
            Some(desc) => desc,
            None => {
                debug_warn!("virtqueue: desc chain writer: no more descriptors");
                return Err(guest_memory::Error::TooLong);
            }
        };

        self.memory.write(desc.gpaddr(), value)?;
        Ok(())
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), guest_memory::Error> {
        // TODO: This assumes the guest provides a descriptor per one write in VMM.
        let desc = match self.descs.pop_front() {
            Some(desc) => desc,
            None => {
                debug_warn!("virtqueue: desc chain writer: no more descriptors");
                return Err(guest_memory::Error::TooLong);
            }
        };

        self.memory.write_bytes(desc.gpaddr(), bytes)?;
        Ok(())
    }
}

pub struct DescChainReader<'a> {
    head: &'a DescChain,
    memory: &'a GuestMemory,
    descs: VecDeque<VirtqDesc>,
}

impl<'a> DescChainReader<'a> {
    pub fn read<T: Copy>(&mut self) -> Result<T, guest_memory::Error> {
        // TODO: This assumes the guest provides a descriptor per one read in VMM.
        let desc = match self.descs.pop_front() {
            Some(desc) => desc,
            None => {
                debug_warn!("virtqueue: desc chain reader: no more descriptors");
                return Err(guest_memory::Error::TooLong);
            }
        };

        let value = self.memory.read(desc.gpaddr())?;
        Ok(value)
    }

    pub fn read_zerocopy(&mut self, len: usize) -> Result<&[u8], guest_memory::Error> {
        // TODO: This assumes the guest provides a descriptor per one read in VMM.
        let desc = match self.descs.pop_front() {
            Some(desc) => desc,
            None => {
                debug_warn!("virtqueue: desc chain reader: no more descriptors");
                return Err(guest_memory::Error::TooLong);
            }
        };

        let slice = self.memory.bytes_slice(desc.gpaddr(), len)?;
        Ok(slice)
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

// TODO: Remove to a common module
fn align_up(size: usize, align: usize) -> usize {
    (size + align - 1) & !(align - 1)
}

pub struct Virtqueue {
    /// Known as *Descriptor Table*.
    desc_gpaddr: GPAddr,
    /// Known as *Available Ring*.
    avail_gpaddr: GPAddr,
    /// Known as *Used Ring*.
    used_gpaddr: GPAddr,
    avail_index: u16,
    used_index: u32,
    num_descs: u32,
    irq_status: u32,
}

impl Virtqueue {
    pub fn new() -> Self {
        Self {
            desc_gpaddr: GPAddr::new(0),
            avail_gpaddr: GPAddr::new(0),
            used_gpaddr: GPAddr::new(0),
            avail_index: 0,
            used_index: 0,
            num_descs: VIRTQUEUE_NUM_DESCS_MAX,
            irq_status: 0,
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
        set_addr(&mut self.used_gpaddr, value, is_high);
    }

    pub fn set_driver_addr(&mut self, value: u32, is_high: bool) {
        set_addr(&mut self.avail_gpaddr, value, is_high);
    }

    pub fn queue_notify(&mut self, memory: &mut GuestMemory, device: &dyn VirtioDevice) {
        while let Some(chain) = self.pop_avail(memory) {
            device.process(memory, self, chain);
        }
    }

    pub fn should_interrupt(&self) -> bool {
        self.irq_status() != 0
    }

    pub fn irq_status(&self) -> u32 {
        self.irq_status
    }

    pub fn acknowledge_irq(&mut self, value: u32) {
        self.irq_status &= !value;
    }

    fn pop_avail(&mut self, memory: &mut GuestMemory) -> Option<DescChain> {
        // TODO: fence here

        let avail = match memory.read::<VirtqAvail>(self.avail_gpaddr) {
            Ok(avail) => avail,
            Err(err) => {
                debug_warn!("virtqueue: pop: failed to read avail ring: {:x?}", err);
                return None;
            }
        };

        if avail.index == self.avail_index {
            debug_warn!("virtqueue: pop: avail index is not updated");
            return None;
        }

        let index_gpaddr = self
            .avail_gpaddr
            .checked_add(size_of::<VirtqAvail>() + self.avail_index as usize * size_of::<u16>())
            .unwrap();

        let desc_index = match memory.read::<u16>(index_gpaddr) {
            Ok(desc_index) => desc_index,
            Err(err) => {
                debug_warn!("virtqueue: pop: failed to read used ring: {:x?}", err);
                return None;
            }
        };

        self.avail_index = (self.avail_index + 1) % (self.num_descs as u16);

        Some(DescChain { head: desc_index })
    }

    pub fn push_used(&mut self, memory: &mut GuestMemory, chain: DescChain, written_len: u32) {
        if chain.head >= self.num_descs as u16 {
            debug_warn!("virtqueue: push_used: chain head is greater than num_descs");
            return;
        }

        let used_index_gpaddr = self
            .used_gpaddr
            .checked_add(offset_of!(VirtqUsed, index))
            .unwrap();
        let used_elem_gpaddr = self
            .used_gpaddr
            .checked_add(
                size_of::<VirtqUsed>() + self.used_index as usize * size_of::<VirtqUsedElem>(),
            )
            .unwrap();

        if let Err(err) = memory.write(
            used_elem_gpaddr,
            VirtqUsedElem {
                id: chain.head as u32,
                len: written_len,
            },
        ) {
            debug_warn!(
                "virtqueue: push_used: failed to write used ring: {:x?}",
                err
            );
            return;
        }

        // This increment must be done before writing the used index.
        self.used_index = (self.used_index + 1) % (self.num_descs as u32);
        self.irq_status |= VIRQ_IRQSTATUS_QUEUE;

        // TODO: fence here

        if let Err(err) = memory.write(used_index_gpaddr, self.used_index) {
            debug_warn!(
                "virtqueue: push_used: failed to write used ring: {:x?}",
                err
            );
        }
    }
}

fn set_addr(gpaddr: &mut GPAddr, value: u32, is_high: bool) {
    let mut addr = gpaddr.as_usize();
    if is_high {
        addr = (addr & 0xffffffff_usize) | ((value as usize) << 32);
    } else {
        addr = (addr & !0xffffffff_usize) | (value as usize);
    }
    *gpaddr = GPAddr::new(addr);
}
