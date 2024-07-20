use core::mem::size_of;
use core::sync::atomic::Ordering;
use core::sync::atomic::{self};

use ftl_api::folio::MmioFolio;
use ftl_api::prelude::Vec;
use ftl_api::types::address::PAddr;
use ftl_api::types::address::VAddr;

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
    folio: MmioFolio,
    index: u16,
    num_descs: u16,
    last_used_index: u16,
    free_head: u16,
    num_free_descs: u16,
    descs: VAddr,
    avail: VAddr,
    used: VAddr,
}

// FIXME:
const PAGE_SIZE: usize = 4096;

pub const fn align_down(value: usize, align: usize) -> usize {
    (value) & !(align - 1)
}

// FIXME:
pub const fn align_up(value: usize, align: usize) -> usize {
    align_down(value + align - 1, align)
}

impl VirtQueue {
    pub fn new(index: u16, transport: &mut dyn VirtioTransport) -> VirtQueue {
        transport.select_queue(index);

        let num_descs = transport.queue_max_size();
        transport.set_queue_size(num_descs);

        let avail_ring_off = size_of::<VirtqDesc>() * (num_descs as usize);
        let avail_ring_size: usize = size_of::<u16>() * (3 + (num_descs as usize));
        let used_ring_off = align_up(avail_ring_off + avail_ring_size, PAGE_SIZE);
        let used_ring_size =
            size_of::<u16>() * 3 + size_of::<VirtqUsedElem>() * (num_descs as usize);
        let virtq_size = used_ring_off + align_up(used_ring_size, PAGE_SIZE);

        let folio = MmioFolio::create(align_up(virtq_size, PAGE_SIZE))
            .expect("failed to allocate virtuqeue");

        let virtqueue_vaddr = folio.vaddr();
        let virtqueue_paddr = folio.paddr();
        let descs = virtqueue_paddr;
        let avail = virtqueue_paddr.add(avail_ring_off);
        let used = virtqueue_paddr.add(used_ring_off);

        transport.set_queue_desc_paddr(descs);
        transport.set_queue_driver_paddr(avail);
        transport.set_queue_device_paddr(used);
        transport.enable_queue();

        // Add descriptors into the free list.
        for i in 0..num_descs {
            let desc =
                unsafe { &mut *virtqueue_vaddr.as_mut_ptr::<VirtqDesc>().offset(i as isize) };
            desc.next = if i == num_descs - 1 { 0 } else { i + 1 };
        }

        VirtQueue {
            folio,
            index,
            num_descs,
            last_used_index: 0,
            free_head: 0,
            num_free_descs: num_descs,
            descs: virtqueue_vaddr.add(0),
            avail: virtqueue_vaddr.add(avail_ring_off),
            used: virtqueue_vaddr.add(used_ring_off),
        }
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
                        debug_assert!(desc.next != 0);
                        desc.next = prev_head;
                        break;
                    }

                    next_desc_index = desc.next;
                }

                self.num_free_descs += num_freed;
                self.last_used_index = self.last_used_index.wrapping_add(1);
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
                VirtqDescBuffer::ReadOnlyFromDevice { paddr: addr, len } => (addr, *len, 0),
                VirtqDescBuffer::WritableFromDevice { paddr: addr, len } => {
                    (addr, *len, VIRTQ_DESC_F_WRITE)
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
        self.avail_mut().index = self.avail_mut().index.wrapping_add(1);
    }

    /// Notifies the device to start processing descriptors.
    pub fn notify(&self, transport: &mut dyn VirtioTransport) {
        atomic::fence(Ordering::Release);
        transport.notify_queue(self.index);
    }

    /// Returns a chain of descriptors processed by the device.
    pub fn pop_used(&mut self) -> Option<VirtqUsedChain> {
        if self.last_used_index == self.used().index {
            return None;
        }

        let head = *self.used_elem(self.last_used_index);
        self.last_used_index = self.last_used_index.wrapping_add(1);

        let mut used_descs = Vec::new();
        let mut next_desc_index = head.id as u16;
        let mut num_descs_in_chain = 1;
        let current_free_head = self.free_head;
        loop {
            let desc = self.desc_mut(next_desc_index);
            used_descs.push(if desc.is_writable() {
                VirtqDescBuffer::WritableFromDevice {
                    paddr: PAddr::new(desc.addr as usize).unwrap(),
                    len: desc.len as usize,
                }
            } else {
                VirtqDescBuffer::ReadOnlyFromDevice {
                    paddr: PAddr::new(desc.addr as usize).unwrap(),
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
        unsafe {
            &mut *self
                .descs
                .as_mut_ptr::<VirtqDesc>()
                .offset((index % self.num_descs) as isize)
        }
    }

    fn avail(&self) -> &VirtqAvail {
        unsafe { &*self.avail.as_ptr::<VirtqAvail>() }
    }

    fn avail_mut(&mut self) -> &mut VirtqAvail {
        unsafe { &mut *self.avail.as_mut_ptr::<VirtqAvail>() }
    }

    fn avail_elem_mut(&mut self, index: u16) -> &mut u16 {
        unsafe {
            &mut *self
                .avail
                .add(size_of::<VirtqAvail>())
                .as_mut_ptr::<u16>()
                .offset((index % self.num_descs) as isize)
        }
    }

    fn used(&self) -> &VirtqUsed {
        unsafe { &*self.used.as_ptr::<VirtqUsed>() }
    }

    fn used_elem(&self, index: u16) -> &VirtqUsedElem {
        unsafe {
            &*self
                .used
                .add(size_of::<VirtqUsed>())
                .as_ptr::<VirtqUsedElem>()
                .offset((index % self.num_descs) as isize)
        }
    }
}
