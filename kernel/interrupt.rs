use starina::poll::Readiness;
use starina_types::error::ErrorCode;
use starina_types::interrupt::Irq;

use crate::arch;
use crate::handle::Handleable;
use crate::poll::Listener;
use crate::poll::Poll;
use crate::refcount::SharedRef;

pub struct Interrupt {
    irq: Irq,
}

impl Interrupt {
    pub fn new(irq: Irq) -> Result<SharedRef<Interrupt>, ErrorCode> {
        let interrupt = SharedRef::new(Interrupt { irq });

        // arch::interrupt_create(&interrupt)?;
        todo!()
        // Ok(interrupt)
    }

    pub fn irq(&self) -> Irq {
        self.irq
    }

    pub fn trigger(&self) -> Result<(), ErrorCode> {
        // self.signal.update(SignalBits::from_raw(1))
        todo!()
    }

    pub fn ack(&self) -> Result<(), ErrorCode> {
        // arch::interrupt_ack(self.irq)
        todo!()
    }
}

impl Handleable for Interrupt {
    fn close(&self) {
        // Do nothing
    }

    fn add_listener(&self, _listener: Listener) -> Result<(), ErrorCode> {
        debug_warn!("unsupported method at {}:{}", file!(), line!());
        Err(ErrorCode::NotSupported)
    }

    fn remove_listener(&self, _poll: &Poll) -> Result<(), ErrorCode> {
        debug_warn!("unsupported method at {}:{}", file!(), line!());
        Err(ErrorCode::NotSupported)
    }

    fn readiness(&self) -> Result<Readiness, ErrorCode> {
        debug_warn!("unsupported method at {}:{}", file!(), line!());
        Err(ErrorCode::NotSupported)
    }
}
