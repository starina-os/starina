use core::mem::offset_of;

use starina::address::GPAddr;
use starina::collections::VecDeque;
use starina::prelude::*;
use starina_utils::endianness::LittleEndian;
use starina_utils::static_assert;

use super::device::VirtioDevice;
use crate::guest_memory;
use crate::guest_memory::GuestMemory;

pub const VIRTQUEUE_NUM_DESCS_MAX: u32 = 256;

const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;

/// Used Buffer Notification.
const VIRQ_IRQSTATUS_QUEUE: u32 = 1 << 0;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VirtqDesc {
    pub addr: LittleEndian<u64>,
    pub len: LittleEndian<u32>,
    pub flags: LittleEndian<u16>,
    pub next: LittleEndian<u16>,
}

static_assert!(size_of::<VirtqDesc>() == 16);

impl VirtqDesc {
    pub fn gpaddr(&self) -> GPAddr {
        GPAddr::new(self.addr.to_host() as usize)
    }

    pub fn is_write_only(&self) -> bool {
        self.flags.to_host() & VIRTQ_DESC_F_WRITE != 0
    }

    pub fn is_read_only(&self) -> bool {
        !self.is_write_only()
    }

    pub fn has_next(&self) -> bool {
        self.flags.to_host() & VIRTQ_DESC_F_NEXT != 0
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
            if desc.is_read_only() {
                readable_descs.push_back(desc);
            } else {
                writable_descs.push_back(desc);
            }

            if !desc.has_next() {
                break;
            }

            desc_index = desc.next.to_host();
        }

        let reader = DescChainReader {
            _head: &self,
            memory,
            descs: readable_descs,
            current: None,
        };

        let writer = DescChainWriter {
            _head: &self,
            memory,
            descs: writable_descs,
        };

        Ok((reader, writer))
    }
}

pub struct DescChainWriter<'a> {
    _head: &'a DescChain,
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
                return Err(guest_memory::Error::OutOfRange);
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
                return Err(guest_memory::Error::OutOfRange);
            }
        };

        self.memory.write_bytes(desc.gpaddr(), bytes)?;
        Ok(())
    }
}

pub struct DescChainReader<'a> {
    _head: &'a DescChain,
    memory: &'a GuestMemory,
    descs: VecDeque<VirtqDesc>,
    current: Option<(VirtqDesc, usize /* offset */)>,
}

impl<'a> DescChainReader<'a> {
    pub fn read<T: Copy>(&mut self) -> Result<T, guest_memory::Error> {
        let read_len = size_of::<T>();
        loop {
            let (desc, offset) = match self.current {
                Some((desc, offset)) => (desc, offset),
                None => {
                    match self.descs.pop_front() {
                        Some(desc) => (desc, 0),
                        None => {
                            debug_warn!("virtqueue: desc chain reader: no more descriptors");
                            return Err(guest_memory::Error::OutOfRange);
                        }
                    }
                }
            };

            let desc_len = desc.len.to_host() as usize;
            if offset + read_len == desc_len {
                // Try next descriptor.
                self.current = None;
                continue;
            }

            if offset + read_len > desc_len {
                debug_warn!(
                    "virtqueue: desc chain reader: tried to read an object which spans across multiple descriptors"
                );
                return Err(guest_memory::Error::OutOfRange);
            }

            let gpaddr = desc
                .gpaddr()
                .checked_add(offset)
                .ok_or(guest_memory::Error::OutOfRange)?;
            let value = self.memory.read(gpaddr)?;
            self.current = Some((desc, offset + read_len));
            return Ok(value);
        }
    }

    pub fn read_zerocopy(&mut self, len: usize) -> Result<&[u8], guest_memory::Error> {
        // TODO: This assumes the guest provides a descriptor per one read in VMM.
        let desc = match self.descs.pop_front() {
            Some(desc) => desc,
            None => {
                debug_warn!("virtqueue: desc chain reader: no more descriptors");
                return Err(guest_memory::Error::OutOfRange);
            }
        };

        let slice = self.memory.bytes_slice(desc.gpaddr(), len)?;
        Ok(slice)
    }

    // TODO: Terrible API name. Consider merging with `read_zerocopy`.
    pub fn read_zerocopy2(&mut self) -> Result<Option<&[u8]>, guest_memory::Error> {
        loop {
            let (desc, offset) = match self.current {
                Some((desc, offset)) => (desc, offset),
                None => {
                    match self.descs.pop_front() {
                        Some(desc) => (desc, 0),
                        None => return Ok(None),
                    }
                }
            };

            self.current = None;
            if offset == desc.len.to_host() as usize {
                continue;
            }

            let gpaddr = desc
                .gpaddr()
                .checked_add(offset)
                .ok_or(guest_memory::Error::OutOfRange)?;

            let len = desc.len.to_host() as usize - offset;
            let slice = self.memory.bytes_slice(gpaddr, len)?;
            return Ok(Some(slice));
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct VirtqAvail {
    flags: LittleEndian<u16>,
    index: LittleEndian<u16>,
    // The rings (an array of descriptor indices) immediately follows here.
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VirtqUsedElem {
    id: LittleEndian<u32>,
    len: LittleEndian<u32>,
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct VirtqUsed {
    flags: LittleEndian<u16>,
    index: LittleEndian<u16>,
    // The rings (an array of VirtqUsedElem) immediately follows here.
}

pub struct Virtqueue {
    /// The queue index.
    index: u32,
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
    pub fn new(index: u32) -> Self {
        Self {
            index,
            desc_gpaddr: GPAddr::new(0),
            avail_gpaddr: GPAddr::new(0),
            used_gpaddr: GPAddr::new(0),
            avail_index: 0,
            used_index: 0,
            num_descs: VIRTQUEUE_NUM_DESCS_MAX,
            irq_status: 0,
        }
    }

    pub fn index(&self) -> u32 {
        self.index
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

        if avail.index.to_host() == self.avail_index {
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
                id: (chain.head as u32).into(),
                len: written_len.into(),
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
