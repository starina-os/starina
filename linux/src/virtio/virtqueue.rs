use core::mem::offset_of;
use core::sync::atomic;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering;

use starina::address::GPAddr;
use starina::collections::VecDeque;
use starina::prelude::*;
use starina::sync::Arc;
use starina_utils::endianness::LittleEndian;
use starina_utils::static_assert;

use super::device::VirtioDevice;
use crate::guest_memory;
use crate::guest_memory::GuestMemory;
use crate::guest_net;

pub const VIRTQUEUE_NUM_DESCS_MAX: u32 = 32;

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
    pub fn split<'a>(
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
            current: None,
            written_len: 0,
        };

        Ok((reader, writer))
    }
}

pub struct DescChainWriter<'a> {
    _head: &'a DescChain,
    memory: &'a GuestMemory,
    descs: VecDeque<VirtqDesc>,
    current: Option<(VirtqDesc, usize /* offset */)>,
    written_len: usize,
}

impl<'a> DescChainWriter<'a> {
    pub fn written_len(&self) -> usize {
        self.written_len
    }

    pub fn write<T: Copy>(&mut self, value: T) -> Result<(), guest_memory::Error> {
        let write_len = size_of::<T>();
        loop {
            let (desc, offset) = match self.current {
                Some((desc, offset)) => (desc, offset),
                None => {
                    match self.descs.pop_front() {
                        Some(desc) => (desc, 0),
                        None => {
                            debug_warn!("virtqueue: desc chain writer: no more descriptors");
                            return Err(guest_memory::Error::OutOfRange);
                        }
                    }
                }
            };

            let desc_len = desc.len.to_host() as usize;
            if offset + write_len > desc_len {
                debug_warn!(
                    "virtqueue: desc chain writer: tried to write an object which spans across multiple descriptors"
                );
                return Err(guest_memory::Error::OutOfRange);
            }

            let gpaddr = desc
                .gpaddr()
                .checked_add(offset)
                .ok_or(guest_memory::Error::OutOfRange)?;
            self.memory.write(gpaddr, value)?;

            self.current = Some((desc, offset + write_len));
            self.written_len += write_len;

            if offset + write_len == desc_len {
                self.current = None;
            }

            return Ok(());
        }
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), guest_memory::Error> {
        let mut remaining = bytes;

        while !remaining.is_empty() {
            let (desc, offset) = match self.current {
                Some((desc, offset)) => (desc, offset),
                None => {
                    match self.descs.pop_front() {
                        Some(desc) => (desc, 0),
                        None => {
                            debug_warn!("virtqueue: desc chain writer: no more descriptors");
                            return Err(guest_memory::Error::OutOfRange);
                        }
                    }
                }
            };

            let desc_len = desc.len.to_host() as usize;
            if offset >= desc_len {
                self.current = None;
                continue;
            }

            let gpaddr = desc
                .gpaddr()
                .checked_add(offset)
                .ok_or(guest_memory::Error::OutOfRange)?;

            let available_len = desc_len - offset;
            let write_len = remaining.len().min(available_len);
            let write_bytes = &remaining[..write_len];

            self.memory.write_bytes(gpaddr, write_bytes)?;
            self.written_len += write_len;
            self.current = Some((desc, offset + write_len));
            if offset + write_len == desc_len {
                self.current = None;
            }

            remaining = &remaining[write_len..];
        }

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

            if offset + read_len == desc_len {
                // Try next descriptor.
                self.current = None;
            }

            return Ok(value);
        }
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<Option<&[u8]>, guest_memory::Error> {
        debug_assert!(len > 0);

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

            let desc_len = desc.len.to_host() as usize;
            if offset >= desc_len {
                self.current = None;
                continue;
            }

            let gpaddr = desc
                .gpaddr()
                .checked_add(offset)
                .ok_or(guest_memory::Error::OutOfRange)?;

            let available_len = desc_len - offset;
            let read_len = len.min(available_len);
            let slice = self.memory.bytes_slice(gpaddr, read_len)?;

            self.current = Some((desc, offset + read_len));
            if offset + read_len == desc_len {
                self.current = None;
            }

            return Ok(Some(slice));
        }
    }

    pub fn read_zerocopy(&mut self, len: usize) -> Result<&[u8], guest_memory::Error> {
        if len == 0 {
            return Ok(&[]);
        }

        match self.read_bytes(len)? {
            Some(slice) => Ok(slice),
            None => {
                debug_warn!("virtqueue: desc chain reader: no more descriptors");
                Err(guest_memory::Error::OutOfRange)
            }
        }
    }
}

impl<'a> guest_net::PacketReader for DescChainReader<'a> {
    fn read_bytes(&mut self, read_len: usize) -> Result<&[u8], guest_memory::Error> {
        self.read_zerocopy(read_len)
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
    used_index: u16,
    num_descs: u32,
    irq_status: Arc<AtomicU32>,
}

impl Virtqueue {
    pub fn new(irq_status: Arc<AtomicU32>, index: u32) -> Self {
        Self {
            index,
            desc_gpaddr: GPAddr::new(0),
            avail_gpaddr: GPAddr::new(0),
            used_gpaddr: GPAddr::new(0),
            avail_index: 0,
            used_index: 0,
            num_descs: VIRTQUEUE_NUM_DESCS_MAX,
            irq_status,
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

    pub fn pop_avail(&mut self, memory: &mut GuestMemory) -> Option<DescChain> {
        atomic::fence(Ordering::Acquire);

        let avail = match memory.read::<VirtqAvail>(self.avail_gpaddr) {
            Ok(avail) => avail,
            Err(err) => {
                debug_warn!("virtqueue: pop: failed to read avail ring: {:x?}", err);
                return None;
            }
        };

        let avail_index_host = avail.index.to_host();
        if avail_index_host == self.avail_index {
            return None;
        }

        // Ensure memory ordering before accessing the ring
        atomic::fence(Ordering::Acquire);

        let index = self.avail_index as usize % self.num_descs as usize;
        let index_gpaddr = self
            .avail_gpaddr
            .checked_add(size_of::<VirtqAvail>() + index * size_of::<u16>())
            .unwrap();

        let desc_index = match memory.read::<u16>(index_gpaddr) {
            Ok(desc_index) => desc_index,
            Err(err) => {
                debug_warn!("virtqueue: pop: failed to read avail ring: {:x?}", err);
                return None;
            }
        };

        // Validate descriptor index is within bounds
        if desc_index >= self.num_descs as u16 {
            debug_warn!(
                "virtqueue[{}]: pop_avail: desc_index {} >= num_descs {}",
                self.index,
                desc_index,
                self.num_descs
            );
            return None;
        }

        self.avail_index = self.avail_index.wrapping_add(1);

        Some(DescChain { head: desc_index })
    }

    pub fn push_desc<F, E>(&mut self, memory: &mut GuestMemory, f: F) -> Result<(), E>
    where
        F: FnOnce(DescChainWriter<'_>) -> Result<usize, E>,
    {
        // FIXME: What if no available descriptor?
        let desc = self.pop_avail(memory).unwrap();
        let (_, writer) = desc.split(self, memory).unwrap();
        // FIXME: Push back to available queue if f returns Err
        let written_len = f(writer)?;

        self.push_used(memory, desc, written_len as u32);
        Ok(())
    }

    pub fn push_used(&mut self, memory: &mut GuestMemory, chain: DescChain, written_len: u32) {
        if chain.head >= self.num_descs as u16 {
            debug_warn!(
                "virtqueue: push_used: chain head {} >= num_descs {}",
                chain.head,
                self.num_descs
            );
            return;
        }

        let used_index_gpaddr = self
            .used_gpaddr
            .checked_add(offset_of!(VirtqUsed, index))
            .unwrap();

        let index = self.used_index as usize % self.num_descs as usize;
        let used_elem_gpaddr = self
            .used_gpaddr
            .checked_add(size_of::<VirtqUsed>() + index * size_of::<VirtqUsedElem>())
            .unwrap();

        let used_elem = VirtqUsedElem {
            id: (chain.head as u32).into(),
            len: written_len.into(),
        };

        if let Err(err) = memory.write(used_elem_gpaddr, used_elem) {
            debug_warn!(
                "virtqueue: push_used: failed to write used ring: {:x?}",
                err
            );
            return;
        }

        self.used_index = self.used_index.wrapping_add(1);
        atomic::fence(Ordering::Release);

        if let Err(err) = memory.write(used_index_gpaddr, self.used_index) {
            debug_warn!(
                "virtqueue: push_used: failed to write used index: {:x?}",
                err
            );
            return;
        }

        self.irq_status
            .fetch_or(VIRQ_IRQSTATUS_QUEUE, Ordering::Relaxed);
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
