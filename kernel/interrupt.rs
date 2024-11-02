use starina_types::error::FtlError;
use starina_types::interrupt::Irq;
use starina_types::signal::SignalBits;

use crate::arch;
use crate::poll::Poller;
use crate::refcount::SharedRef;
use crate::signal::Signal;

pub struct Interrupt {
    irq: Irq,
    signal: SharedRef<Signal>,
}

impl Interrupt {
    pub fn new(irq: Irq) -> Result<SharedRef<Interrupt>, FtlError> {
        let signal = Signal::new()?;
        let interrupt = SharedRef::new(Interrupt { irq, signal });

        arch::interrupt_create(&interrupt)?;
        Ok(interrupt)
    }

    pub fn irq(&self) -> Irq {
        self.irq
    }

    pub fn add_poller(&self, poller: SharedRef<Poller>) {
        self.signal.add_poller(poller);
    }

    pub fn remove_poller(&self, poller: &SharedRef<Poller>) {
        self.signal.remove_poller(poller);
    }

    pub fn trigger(&self) -> Result<(), FtlError> {
        self.signal.update(SignalBits::from_raw(1))
    }

    pub fn ack(&self) -> Result<(), FtlError> {
        arch::interrupt_ack(self.irq)
    }
}
