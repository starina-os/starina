use alloc::vec::Vec;

use ftl_types::error::FtlError;
use ftl_types::poll::PollEvent;
use ftl_types::signal::SignalBits;

use crate::poll::Poller;
use crate::ref_counted::SharedRef;
use crate::spinlock::SpinLock;

struct Mutable {
    pollers: Vec<SharedRef<Poller>>,
    pending: SignalBits,
}

pub struct Signal {
    mutable: SpinLock<Mutable>,
}

impl Signal {
    pub fn new() -> Result<SharedRef<Signal>, FtlError> {
        let signal = Signal {
            mutable: SpinLock::new(Mutable {
                pollers: Vec::new(),
                pending: SignalBits::empty(),
            }),
        };

        Ok(SharedRef::new(signal))
    }

    pub fn add_poller(&self, poller: SharedRef<Poller>) {
        let mut mutable = self.mutable.lock();

        if !mutable.pending.is_empty() {
            poller.set_ready(PollEvent::READABLE);
        }

        mutable.pollers.push(poller);
    }

    pub fn remove_poller(&self, poller: &SharedRef<Poller>) {
        let mut mutable = self.mutable.lock();
        mutable.pollers.retain(|p| !SharedRef::ptr_eq(p, poller));
    }

    pub fn update(&self, value: SignalBits) -> Result<(), FtlError> {
        let mut mutable = self.mutable.lock();
        mutable.pending |= value;

        // TODO: EPOLLEXCLUSIVE-like behavior to prevent thundering herd
        for poller in &mutable.pollers {
            poller.set_ready(PollEvent::READABLE);
        }

        Ok(())
    }

    pub fn clear(&self) -> Result<SignalBits, FtlError> {
        let mut mutable = self.mutable.lock();
        let value = mutable.pending.clear();
        if value.is_empty() {
            return Err(FtlError::WouldBlock);
        }

        Ok(value)
    }
}
