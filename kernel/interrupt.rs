use ftl_types::error::FtlError;
use ftl_types::interrupt::Irq;
use ftl_types::signal::SignalBits;

use crate::arch;
use crate::poll::Poller;
use crate::ref_counted::SharedRef;
use crate::signal::Signal;

pub struct Interrupt {
    irq: Irq,
    signal: SharedRef<Signal>,
}

impl Interrupt {
    pub fn new(irq: Irq) -> Result<SharedRef<Interrupt>, FtlError> {

        let signal = Signal::new()?;
        let interrupt = SharedRef::new(Interrupt {
            irq,
            signal
        });

        arch::create_interrupt(&interrupt)?;
        Ok(interrupt)
    }

    pub fn irq(&self) -> Irq {
        self.irq
    }

    pub fn add_poller(&self, poller: SharedRef<Poller>) {
        self.signal.add_poller(poller);
    }

    pub fn trigger(&self) -> Result<(), FtlError> {
        self.signal.update(SignalBits::from_raw(1))
    }

    pub fn ack(&self) -> Result<(), FtlError> {
        arch::ack_interrupt(self.irq)
    }
}
