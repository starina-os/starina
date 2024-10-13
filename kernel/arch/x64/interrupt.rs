use alloc::collections::BTreeMap;

use ftl_types::error::FtlError;
use ftl_types::interrupt::Irq;

use super::io_apic::IO_APIC;
use super::local_apic::LOCAL_APIC;
use crate::interrupt::Interrupt;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;

static LISTENERS: SpinLock<BTreeMap<Irq, SharedRef<Interrupt>>> = SpinLock::new(BTreeMap::new());

pub fn interrupt_create(interrupt: &SharedRef<Interrupt>) -> Result<(), FtlError> {
    let irq = interrupt.irq();
    IO_APIC.lock().as_mut().unwrap().enable_irq(irq);
    LISTENERS.lock().insert(irq, interrupt.clone());
    Ok(())
}

pub fn interrupt_ack(irq: Irq) -> Result<(), FtlError> {
    /* Nothing to do: per-IRQ ack is needed for IO APIC */
    Ok(())
}

pub fn handle_interrupt(vector: usize) {
    let irq = Irq::from_raw(vector as usize);

    let listeners = LISTENERS.lock();
    if let Some(listener) = listeners.get(&irq) {
        listener.trigger().unwrap();
    }
}
