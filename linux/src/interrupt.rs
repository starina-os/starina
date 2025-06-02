use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering;

use starina::sync::Arc;

#[derive(Clone)]
pub struct IrqTrigger {
    irqs: Arc<AtomicU32>,
}

impl IrqTrigger {
    pub fn new() -> Self {
        Self {
            irqs: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn trigger(&self, irq: u8) {
        debug_assert!(irq < 32);

        self.irqs.fetch_or(1 << irq, Ordering::Relaxed);
    }

    pub fn clear_all(&self) -> u32 {
        self.irqs.swap(0, Ordering::Relaxed)
    }
}
