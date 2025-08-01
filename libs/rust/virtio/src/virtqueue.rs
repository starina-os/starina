use core::fmt;
use core::mem::size_of;
use core::sync::atomic;
use core::sync::atomic::Ordering;

use starina::address::PAddr;
use starina::folio::page_size;
use starina::mmio::MmioRegion;
use starina::prelude::*;
use starina_utils::alignment::align_up;

use super::transports::VirtioTransport;

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

#[derive(Debug)]
pub enum VirtqDescBuffer {
    ReadOnlyFromDevice { paddr: PAddr, len: usize },
    WritableFromDevice { paddr: PAddr, len: usize },
}

pub struct VirtqUsedChain {
    pub descs: Vec<VirtqDescBuffer>,
    pub total_len: usize,
}

/// A virtqueue.
pub struct VirtQueue {
    #[allow(dead_code)]
    folio: MmioRegion,
    index: u16,
    num_descs: u16,
    last_used_index: u16,
    free_head: u16,
    num_free_descs: u16,
    avail_ring_off: usize,
    used_ring_off: usize,
}

impl VirtQueue {
    pub fn new(index: u16, transport: &mut dyn VirtioTransport) -> VirtQueue {
        transport.select_queue(index);

        let num_descs = transport.queue_max_size();
        transport.set_queue_size(num_descs);

        let avail_ring_off = size_of::<VirtqDesc>() * (num_descs as usize);
        let avail_ring_size: usize = size_of::<u16>() * (3 + (num_descs as usize));
        let used_ring_off = align_up(avail_ring_off + avail_ring_size, page_size());
        let used_ring_size =
            size_of::<u16>() * 3 + size_of::<VirtqUsedElem>() * (num_descs as usize);
        let virtq_size = used_ring_off + align_up(used_ring_size, page_size());

        let folio = MmioRegion::create(virtq_size).expect("failed to allocate virtuqeue");
        let descs = folio.paddr();
        let avail = folio.paddr().add(avail_ring_off);
        let used = folio.paddr().add(used_ring_off);

        transport.set_queue_desc_paddr(descs);
        transport.set_queue_driver_paddr(avail);
        transport.set_queue_device_paddr(used);
        transport.enable_queue();

        let mut this = VirtQueue {
            folio,
            index,
            num_descs,
            last_used_index: 0,
            free_head: 0,
            num_free_descs: num_descs,
            avail_ring_off,
            used_ring_off,
        };

        // Add descriptors into the free list.
        for i in 0..num_descs {
            this.desc_mut(i).next = if i == num_descs - 1 { 0 } else { i + 1 };
        }

        this
    }

    /// Enqueues a request to the device. A request is a chain of descriptors
    /// (e.g. `struct virtio_blk_req` as defined in the spec).
    ///
    /// Once you've enqueued all requests, you need to notify the device through
    /// the `notify` method.
    pub fn enqueue(&mut self, chain: &[VirtqDescBuffer]) {
        debug_assert!(!chain.is_empty());

        // Try freeing used descriptors.
        if (self.num_free_descs as usize) < chain.len() {
            while self.last_used_index != self.used().index {
                let used_elem_index = self.used_elem(self.last_used_index).id as u16;

                // Enqueue the popped chain back into the free list.
                let prev_head = self.free_head;
                self.free_head = used_elem_index;

                // Count the number of descriptors in the chain.
                let mut num_freed = 0;
                let mut next_desc_index = used_elem_index;
                loop {
                    let desc = self.desc_mut(next_desc_index);
                    num_freed += 1;

                    if (desc.flags & VIRTQ_DESC_F_NEXT) == 0 {
                        desc.next = prev_head;
                        break;
                    }

                    next_desc_index = desc.next;
                }

                self.num_free_descs += num_freed;
                self.last_used_index = (self.last_used_index + 1) % self.num_descs;
            }
        }

        // Check if we have the enough number of free descriptors.
        if (self.num_free_descs as usize) < chain.len() {
            panic!("not enough descs for {}!", self.index);
        }

        let head_index = self.free_head;
        let mut desc_index = self.free_head;
        for (i, buffer) in chain.iter().enumerate() {
            let desc = self.desc_mut(desc_index);
            let (addr, len, flags) = match buffer {
                VirtqDescBuffer::ReadOnlyFromDevice { paddr, len } => (paddr, *len, 0),
                VirtqDescBuffer::WritableFromDevice { paddr, len } => {
                    (paddr, *len, VIRTQ_DESC_F_WRITE)
                }
            };

            desc.addr = addr.as_usize() as u64;
            desc.len = len.try_into().unwrap();
            desc.flags = flags;

            if i == chain.len() - 1 {
                let unused_next = desc.next;
                desc.next = 0;
                desc.flags &= !VIRTQ_DESC_F_NEXT;
                self.free_head = unused_next;
                self.num_free_descs -= chain.len() as u16;
            } else {
                desc.flags |= VIRTQ_DESC_F_NEXT;
                desc_index = desc.next;
            }
        }

        let avail_elem_index = self.avail().index & (self.num_descs() - 1);
        *self.avail_elem_mut(avail_elem_index) = head_index;

        let avail = self.avail_mut();
        avail.index = avail.index.wrapping_add(1);
    }

    /// Notifies the device to start processing descriptors.
    pub fn notify(&self, transport: &mut dyn VirtioTransport) {
        atomic::fence(Ordering::Release);
        transport.notify_queue(self.index);
    }

    /// Returns a chain of descriptors processed by the device.
    pub fn pop_used(&mut self) -> Option<VirtqUsedChain> {
        // TODO: Shouldn't we use atomic read here?
        if self.last_used_index == self.used().index {
            return None;
        }

        let head = *self.used_elem(self.last_used_index);
        self.last_used_index = (self.last_used_index + 1) % self.num_descs;

        let mut used_descs = Vec::new();
        let mut next_desc_index = head.id as u16;
        let mut num_descs_in_chain = 1;
        let current_free_head = self.free_head;
        loop {
            let desc = self.desc_mut(next_desc_index);
            used_descs.push(if desc.is_writable() {
                VirtqDescBuffer::WritableFromDevice {
                    paddr: PAddr::new(desc.addr as usize),
                    len: desc.len as usize,
                }
            } else {
                VirtqDescBuffer::ReadOnlyFromDevice {
                    paddr: PAddr::new(desc.addr as usize),
                    len: desc.len as usize,
                }
            });

            if !desc.has_next() {
                // Prepend the popped chain into the free list.
                desc.next = current_free_head;
                self.free_head = head.id as u16;
                self.num_free_descs += num_descs_in_chain;
                break;
            }

            next_desc_index = desc.next;
            num_descs_in_chain += 1;
        }

        Some(VirtqUsedChain {
            total_len: head.len as usize,
            descs: used_descs,
        })
    }

    /// Returns the defined number of descriptors in the virtqueue.
    pub fn num_descs(&self) -> u16 {
        self.num_descs
    }

    fn desc_mut(&mut self, index: u16) -> &mut VirtqDesc {
        let i = (index % self.num_descs) as usize;
        let offset = i * size_of::<VirtqDesc>();
        unsafe { &mut *self.folio.as_mut(offset) }
    }

    fn avail(&self) -> &VirtqAvail {
        unsafe { self.folio.as_ref(self.avail_ring_off) }
    }

    fn avail_mut(&mut self) -> &mut VirtqAvail {
        unsafe { &mut *self.folio.as_mut(self.avail_ring_off) }
    }

    fn avail_elem_mut(&mut self, index: u16) -> &mut u16 {
        let i = (index % self.num_descs) as usize;
        let offset = self.avail_ring_off + size_of::<VirtqAvail>() + i * size_of::<u16>();
        unsafe { &mut *self.folio.as_mut(offset) }
    }

    fn used(&self) -> &VirtqUsed {
        unsafe { self.folio.as_ref(self.used_ring_off) }
    }

    fn used_elem(&self, index: u16) -> &VirtqUsedElem {
        let i = (index % self.num_descs) as usize;
        let offset = self.used_ring_off + size_of::<VirtqUsed>() + i * size_of::<VirtqUsedElem>();
        unsafe { self.folio.as_ref(offset) }
    }
}

impl fmt::Debug for VirtQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VirtQueue")
            .field("index", &self.index)
            .finish()
    }
}
