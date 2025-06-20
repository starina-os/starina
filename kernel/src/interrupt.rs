use starina::interrupt::IrqMatcher;
use starina::poll::Readiness;
use starina_types::error::ErrorCode;
use starina_types::interrupt::Irq;

use crate::arch::INTERRUPT_CONTROLLER;
use crate::handle::Handleable;
use crate::poll::Listener;
use crate::poll::ListenerSet;
use crate::poll::Poll;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;

struct Mutable {
    listeners: ListenerSet,
    active: bool,
}

pub struct Interrupt {
    irq: Irq,
    mutable: SpinLock<Mutable>,
}

impl Interrupt {
    pub fn attach(irq_matcher: IrqMatcher) -> Result<SharedRef<Interrupt>, ErrorCode> {
        let irq = INTERRUPT_CONTROLLER.acquire_irq(irq_matcher)?;

        let interrupt = SharedRef::new(Interrupt {
            irq,
            mutable: SpinLock::new(Mutable {
                listeners: ListenerSet::new(),
                active: false,
            }),
        })?;

        INTERRUPT_CONTROLLER.enable_irq(interrupt.clone());
        Ok(interrupt)
    }

    pub fn irq(&self) -> Irq {
        self.irq
    }

    pub fn trigger(&self) -> Result<(), ErrorCode> {
        let mut mutable = self.mutable.lock();
        mutable.active = true;
        mutable.listeners.notify_all(Readiness::READABLE);
        Ok(())
    }

    pub fn acknowledge(&self) -> Result<(), ErrorCode> {
        let mut mutable = self.mutable.lock();
        mutable.active = false;
        drop(mutable);

        INTERRUPT_CONTROLLER.acknowledge_irq(self.irq);
        Ok(())
    }
}

impl Handleable for Interrupt {
    fn close(&self) {
        let mutable = self.mutable.lock();
        mutable.listeners.notify_all(Readiness::CLOSED);
        drop(mutable);

        INTERRUPT_CONTROLLER.disable_irq(self.irq);
    }

    fn add_listener(&self, listener: Listener) -> Result<(), ErrorCode> {
        self.mutable.lock().listeners.add_listener(listener)?;
        Ok(())
    }

    fn remove_listener(&self, poll: &Poll) -> Result<(), ErrorCode> {
        self.mutable.lock().listeners.remove_listener(poll);
        Ok(())
    }

    fn readiness(&self) -> Result<Readiness, ErrorCode> {
        let mut readiness = Readiness::new();
        let active = self.mutable.lock().active;
        if active {
            readiness |= Readiness::READABLE;
        }

        Ok(readiness)
    }
}
