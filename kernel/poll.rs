use core::sync::atomic::AtomicU8;
use core::sync::atomic::Ordering;

use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::poll::PollEvent;
use hashbrown::HashMap;

use crate::handle::AnyHandle;
use crate::interrupt;
use crate::ref_counted::SharedRef;
use crate::sleep::SleepCallbackResult;
use crate::sleep::SleepPoint;
use crate::spinlock::SpinLock;

pub struct Poller {
    sleep_point: SharedRef<SleepPoint>,
    handle_id: HandleId,
    interests: PollEvent,
    ready: AtomicU8,
}

impl Poller {
    pub fn set_ready(&self, ready: PollEvent) {
        let intersects = ready & self.interests;
        if intersects.is_empty() {
            return;
        }

        self.ready.fetch_or(intersects.as_raw(), Ordering::SeqCst); // TODO: correct ordering
        self.sleep_point.wake_all();
    }
}

impl Drop for Poller {
    fn drop(&mut self) {
        self.set_ready(PollEvent::CLOSED);
    }
}

pub struct Poll {
    entries: SpinLock<HashMap<HandleId, SharedRef<Poller>>>,
    sleep_point: SharedRef<SleepPoint>,
}

impl Poll {
    pub fn new() -> SharedRef<Poll> {
        let poll = Poll {
            entries: SpinLock::new(HashMap::new()),
            sleep_point: SharedRef::new(SleepPoint::new()),
        };

        SharedRef::new(poll)
    }

    pub fn add(&self, object: &AnyHandle, object_id: HandleId, interests: PollEvent) {
        let poller = SharedRef::new(Poller {
            sleep_point: self.sleep_point.clone(),
            handle_id: object_id,
            interests,
            ready: AtomicU8::new(0),
        });

        match object {
            AnyHandle::Channel(ch) => {
                ch.add_poller(poller.clone());
            }
            AnyHandle::Interrupt(interrupt) => {
                interrupt.add_poller(poller.clone());
            }
            _ => {
                todo!(); // TODO: support other handle types
            }
        }

        self.entries.lock().insert(object_id, poller);
    }

    pub fn wait(&self) -> Result<(PollEvent, HandleId), FtlError> {
        self.sleep_point.sleep_loop(&self.entries, |entries| {
            for entry in entries.values() {
                let raw_ready = entry.ready.swap(0, Ordering::SeqCst); // TODO: correct ordering
                let ready = PollEvent::from_raw(raw_ready);
                if ready.is_empty() {
                    continue;
                }

                let handle_id = entry.handle_id;
                if ready.contains(PollEvent::CLOSED) {
                    entries.remove(&handle_id);
                }

                return SleepCallbackResult::Ready(Ok((ready, handle_id)));
            }

            SleepCallbackResult::Sleep
        })
    }
}
