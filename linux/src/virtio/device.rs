use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering;

use starina::prelude::*;
use starina::sync::Arc;
use starina::sync::Mutex;

use super::virtqueue::DescChain;
use super::virtqueue::Virtqueue;
use crate::guest_memory::GuestMemory;
use crate::interrupt::IrqTrigger;
use crate::mmio;
use crate::virtio::virtqueue::VIRTQUEUE_NUM_DESCS_MAX;

/// The host-side (device-side) of a virtio device.
///
/// Guest OS interacts with this device through their virtio
/// device drivers.
pub trait VirtioDevice {
    #[allow(unused_variables)]
    fn connect_to_guest(
        &self,
        memory: &mut GuestMemory,
        vq: &mut Virtqueue,
        guest_port: u16,
        proto: crate::guest_net::IpProto,
        forwarder: Box<dyn FnMut(&crate::guest_net::ConnKey, &[u8])>,
    ) -> crate::guest_net::ConnKey {
        // FIXME: This is virtio-net specific.
        unimplemented!()
    }
    #[allow(unused_variables)]
    fn flush_to_guest(&self, memory: &mut GuestMemory, vq: &mut Virtqueue) {
        // FIXME: This is virtio-net specific.
    }

    #[allow(unused_variables)]
    fn send_to_guest(
        &self,
        memory: &mut GuestMemory,
        vq: &mut Virtqueue,
        connkey: &crate::guest_net::ConnKey,
        payload: &[u8],
    ) {
        // FIXME: This is virtio-net specific.
        unimplemented!()
    }

    fn num_queues(&self) -> u32;
    fn device_features(&self) -> u64;
    fn device_id(&self) -> u32;
    fn vendor_id(&self) -> u32;
    fn config_read(&self, offset: u64, buf: &mut [u8]);
    fn process(&self, memory: &mut GuestMemory, vq: &mut Virtqueue, chain: DescChain);
}

pub const VIRTIO_MMIO_SIZE: usize = 4096;

// Virtio MMIO registers.
// <https://docs.oasis-open.org/virtio/virtio/v1.3/csd01/virtio-v1.3-csd01.html#:~:text=42%3E%3B%C2%A0%0A%7D-,4.2.2%20MMIO%20Device%20Register%20Layout,-MMIO%20virtio%20devices>
const REG_MAGIC: u64 = 0x00;
const REG_VERSION: u64 = 0x04;
const REG_DEVICE_ID: u64 = 0x08;
const REG_VENDOR_ID: u64 = 0x0c;
const REG_DEVICE_FEATURES: u64 = 0x10;
const REG_DEVICE_FEATURES_SEL: u64 = 0x14;
const REG_DRIVER_FEATURES: u64 = 0x20;
const REG_DRIVER_FEATURES_SEL: u64 = 0x24;
const REG_QUEUE_SELECT: u64 = 0x30;
const REG_QUEUE_SIZE_MAX: u64 = 0x34;
const REG_QUEUE_SIZE: u64 = 0x38;
const REG_QUEUE_READY: u64 = 0x44;
const REG_QUEUE_NOTIFY: u64 = 0x50;
const REG_INTERRUPT_STATUS: u64 = 0x60;
const REG_INTERRUPT_ACK: u64 = 0x64;
const REG_DEVICE_STATUS: u64 = 0x70;
const REG_QUEUE_DESC_LOW: u64 = 0x80;
const REG_QUEUE_DESC_HIGH: u64 = 0x84;
const REG_QUEUE_DRIVER_LOW: u64 = 0x90;
const REG_QUEUE_DRIVER_HIGH: u64 = 0x94;
const REG_QUEUE_DEVICE_LOW: u64 = 0xa0;
const REG_QUEUE_DEVICE_HIGH: u64 = 0xa4;
const REG_QUEUE_CONFIG_GEN: u64 = 0xfc;
const REG_CONFIG_START: u64 = 0x100;

const VIRTIO_F_VERSION_1: u64 = 1 << 32;

struct Mutable {
    device_features_select: u32,
    driver_features_select: u32,
    device_status: u32,
    driver_features: u64,
    queue_select: u32,
    queues: Vec<Virtqueue>,
}

/// Virtio device over memory-mapped I/O.
pub struct VirtioMmio {
    irq: u8,
    irq_status: Arc<AtomicU32>,
    irq_trigger: IrqTrigger,
    device: Box<dyn VirtioDevice>,
    mutable: Mutex<Mutable>,
}

impl VirtioMmio {
    pub fn new<D: VirtioDevice + 'static>(irq_trigger: IrqTrigger, irq: u8, device: D) -> Self {
        let irq_status = Arc::new(AtomicU32::new(0));
        let num_queues = device.num_queues();
        let mut queues = Vec::with_capacity(num_queues as usize);
        for i in 0..num_queues {
            queues.push(Virtqueue::new(irq_status.clone(), i));
        }

        Self {
            irq,
            irq_status,
            irq_trigger,
            device: Box::new(device),
            mutable: Mutex::new(Mutable {
                device_features_select: 0,
                driver_features_select: 0,
                device_status: 0,
                driver_features: 0,
                queue_select: 0,
                queues,
            }),
        }
    }

    pub fn use_vq<T>(
        &self,
        index: u32,
        f: impl FnOnce(&dyn VirtioDevice, &mut Virtqueue) -> T,
    ) -> T {
        let mut mutable = self.mutable.lock();
        let vq = mutable.queues.get_mut(index as usize).unwrap();

        let ret = f(&*self.device, vq);

        if self.irq_status.load(Ordering::Relaxed) != 0 {
            self.irq_trigger.trigger(self.irq);
        }

        ret
    }
}

impl mmio::Device for VirtioMmio {
    fn mmio_read(
        &self,
        _memory: &mut GuestMemory,
        offset: u64,
        dst: &mut [u8],
    ) -> Result<(), mmio::Error> {
        // trace!(
        //     "virtio mmio read: offset={:x}, width={:x}",
        //     offset,
        //     dst.len()
        // );

        if offset >= REG_CONFIG_START {
            let config_offset = offset - REG_CONFIG_START;
            self.device.config_read(config_offset, dst);
            return Ok(());
        }

        let width = dst.len();
        let mutable = self.mutable.lock();
        match width {
            4 => {
                let value = match offset {
                    REG_MAGIC => 0x74726976,
                    REG_VERSION => 2,
                    REG_DEVICE_ID => self.device.device_id(),
                    REG_VENDOR_ID => self.device.vendor_id(),
                    REG_DEVICE_FEATURES => {
                        let features = self.device.device_features() | VIRTIO_F_VERSION_1;
                        if mutable.device_features_select == 0 {
                            (features & 0xffffffff) as u32
                        } else {
                            (features >> 32) as u32
                        }
                    }
                    REG_DEVICE_FEATURES_SEL => mutable.device_features_select,
                    REG_DEVICE_STATUS => mutable.device_status,
                    REG_QUEUE_READY => 0,
                    REG_QUEUE_SIZE_MAX => VIRTQUEUE_NUM_DESCS_MAX,
                    REG_QUEUE_CONFIG_GEN => 0,
                    REG_INTERRUPT_STATUS => self.irq_status.load(Ordering::Relaxed),
                    _ => {
                        panic!(
                            "unexpected virtio-mmio read: offset={:x}, width={}",
                            offset, width
                        );
                    }
                };

                dst.copy_from_slice(&value.to_ne_bytes());
            }
            _ => {
                panic!("unsupported virtio-mmio read width: {:x}", width);
            }
        }

        if self.irq_status.load(Ordering::Relaxed) != 0 {
            self.irq_trigger.trigger(self.irq);
        }

        Ok(())
    }

    fn mmio_write(
        &self,
        memory: &mut GuestMemory,
        offset: u64,
        src: &[u8],
    ) -> Result<(), mmio::Error> {
        // trace!(
        //     "virtio mmio write: offset={:x}, width={:x}",
        //     offset,
        //     src.len()
        // );

        let width = src.len();
        if width != 4 {
            panic!(
                "unexpected virtio-mmio write: offset={:x}, width={}",
                offset, width
            );
        }

        let value = u32::from_ne_bytes(src.try_into().unwrap());
        let mut mutable = self.mutable.lock();
        match offset {
            REG_DEVICE_FEATURES_SEL => {
                mutable.device_features_select = value;
            }
            REG_DEVICE_STATUS => {
                mutable.device_status = value;
            }
            REG_DRIVER_FEATURES_SEL => {
                mutable.driver_features_select = value;
            }
            REG_DRIVER_FEATURES => {
                if mutable.driver_features_select == 0 {
                    mutable.driver_features = value as u64;
                } else {
                    mutable.driver_features &= 0xffffffff;
                    mutable.driver_features |= (value as u64) << 32;
                }
            }
            REG_QUEUE_SELECT => {
                mutable.queue_select = value;
            }
            REG_QUEUE_SIZE => {
                let queue_index = mutable.queue_select as usize;
                mutable
                    .queues
                    .get_mut(queue_index)
                    .expect("queue index out of range")
                    .set_queue_size(value);
            }
            REG_QUEUE_READY => {}
            REG_QUEUE_NOTIFY => {
                let queue_index = mutable.queue_select as usize;
                mutable
                    .queues
                    .get_mut(queue_index)
                    .expect("queue index out of range")
                    .queue_notify(memory, &*self.device);
            }
            REG_INTERRUPT_ACK => {
                self.irq_status.fetch_and(0, Ordering::Relaxed);
            }
            REG_QUEUE_DESC_LOW | REG_QUEUE_DESC_HIGH => {
                let queue_index = mutable.queue_select as usize;
                mutable
                    .queues
                    .get_mut(queue_index)
                    .expect("queue index out of range")
                    .set_desc_addr(value, offset == REG_QUEUE_DESC_HIGH);
            }
            REG_QUEUE_DRIVER_LOW | REG_QUEUE_DRIVER_HIGH => {
                let queue_index = mutable.queue_select as usize;
                mutable
                    .queues
                    .get_mut(queue_index)
                    .expect("queue index out of range")
                    .set_driver_addr(value, offset == REG_QUEUE_DRIVER_HIGH);
            }
            REG_QUEUE_DEVICE_LOW | REG_QUEUE_DEVICE_HIGH => {
                let queue_index = mutable.queue_select as usize;
                mutable
                    .queues
                    .get_mut(queue_index)
                    .expect("queue index out of range")
                    .set_device_addr(value, offset == REG_QUEUE_DEVICE_HIGH);
            }
            _ => {
                panic!(
                    "unexpected virtio-mmio write: offset={:x}, width={}",
                    offset, width
                );
            }
        }

        if self.irq_status.load(Ordering::Relaxed) != 0 {
            self.irq_trigger.trigger(self.irq);
        }

        Ok(())
    }
}
