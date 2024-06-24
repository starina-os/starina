use alloc::collections::VecDeque;
use alloc::vec::Vec;

use ftl_inlinedvec::InlinedVec;
use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::message::MessageInfo;
use ftl_types::message::MESSAGE_DATA_MAX_LEN;
use ftl_types::message::MESSAGE_HANDLES_MAX_COUNT;

use crate::cpuvar::current_thread;
use crate::handle::AnyHandle;
use crate::poll::PollPoint;
use crate::poll::PollResult;
use crate::ref_counted::SharedRef;
use crate::spinlock::SpinLock;

struct MessageEntry {
    msginfo: MessageInfo,
    data: Vec<u8>,
    handles: InlinedVec<AnyHandle, MESSAGE_HANDLES_MAX_COUNT>,
}

struct Mutable {
    peer: Option<SharedRef<Channel>>,
    queue: VecDeque<MessageEntry>,
}

pub struct Channel {
    mutable: SpinLock<Mutable>,
    event_point: PollPoint,
}

impl Channel {
    pub fn new() -> Result<(SharedRef<Channel>, SharedRef<Channel>), FtlError> {
        let ch0 = SharedRef::new(Channel {
            event_point: PollPoint::new(),
            mutable: SpinLock::new(Mutable {
                peer: None,
                queue: VecDeque::new(),
            }),
        });
        let ch1 = SharedRef::new(Channel {
            event_point: PollPoint::new(),
            mutable: SpinLock::new(Mutable {
                peer: None,
                queue: VecDeque::new(),
            }),
        });

        // TODO: Can we avoid this mutate-after-construct?
        ch0.mutable.lock().peer = Some(ch1.clone());
        ch1.mutable.lock().peer = Some(ch0.clone());

        Ok((ch0, ch1))
    }

    pub fn send(
        &self,
        msginfo: MessageInfo,
        buf: &[u8; MESSAGE_DATA_MAX_LEN],
        handles: &[HandleId; MESSAGE_HANDLES_MAX_COUNT],
    ) -> Result<(), FtlError> {
        let mutable = self.mutable.lock();
        let peer_ch = mutable.peer.as_ref().ok_or(FtlError::NoPeer)?;

        // Move handles.
        //
        // In this phase, since we don't know the receiver process, we don't
        // move to the desination process, but keep ownership of them (AnyHandle)
        // in the message entry.
        let num_handles = msginfo.num_handles();
        let mut moved_handles = InlinedVec::new();
        if num_handles > 0 {
            let current_thread = current_thread();

            // Note: Don't release this lock until we've moved all handles
            //       to guarantee that the second loop never fails.
            let mut our_handles = current_thread.process().handles().lock();

            // First loop: make sure moving handles won't fail.
            for i in 0..num_handles {
                if !our_handles.is_movable(handles[i]) {
                    return Err(FtlError::HandleNotMovable);
                }
            }

            // Second loop: Remove handles from the current process.
            for i in 0..num_handles {
                // SAFETY: unwrap() won't panic because we've checked the handle
                //         is movable in the previous loop.
                let handle = our_handles.remove(handles[i]).unwrap();

                // SAFETY: unwrap() won't panic because `handles` should have
                //         enough capacity up to MESSAGE_HANDLES_MAX_COUNT.
                moved_handles.try_push(handle).unwrap();
            }
        }

        // Copy message data into the kernel memory.
        let data_len = msginfo.data_len();
        let data = buf[0..data_len].to_vec();

        let entry = MessageEntry {
            msginfo,
            data,
            handles: moved_handles,
        };

        let mut peer_mutable = peer_ch.mutable.lock();
        peer_mutable.queue.push_back(entry);
        peer_ch.event_point.wake();

        Ok(())
    }

    pub fn recv(
        &self,
        buf: &mut [u8; MESSAGE_DATA_MAX_LEN],
        handles: &mut [HandleId; MESSAGE_HANDLES_MAX_COUNT],
    ) -> Result<MessageInfo, FtlError> {
        let mut entry = self.event_point.poll_loop(&self.mutable, |mutable| {
            if let Some(entry) = mutable.queue.pop_front() {
                return PollResult::Ready(entry);
            }

            PollResult::Sleep
        });

        // Install handles into the current (receiver) process.
        let current_thread = current_thread();
        let mut handle_table = current_thread.process().handles().lock();
        for (i, any_handle) in entry.handles.drain(..).enumerate() {
            // TODO: Define the expected behavior when it fails to add a handle.
            handles[i] = handle_table.add(any_handle)?;
        }

        let data_len = entry.msginfo.data_len();
        buf[0..data_len].copy_from_slice(&entry.data[0..data_len]);
        Ok(entry.msginfo)
    }
}
