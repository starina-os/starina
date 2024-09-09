use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::fmt;
use core::mem::offset_of;

use ftl_inlinedvec::InlinedVec;
use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::message::MessageBuffer;
use ftl_types::message::MessageInfo;
use ftl_types::message::MESSAGE_HANDLES_MAX_COUNT;
use ftl_types::poll::PollEvent;

use crate::cpuvar::current_thread;
use crate::handle::AnyHandle;
use crate::poll::Poller;
use crate::process::Process;
use crate::ref_counted::SharedRef;
use crate::spinlock::SpinLock;
use crate::thread::Continuation;
use crate::thread::Thread;
use crate::uaddr::UAddr;
use crate::wait_queue::WaitQueue;

struct MessageEntry {
    msginfo: MessageInfo,
    data: Vec<u8>,
    handles: InlinedVec<AnyHandle, MESSAGE_HANDLES_MAX_COUNT>,
}

struct Mutable {
    peer: Option<SharedRef<Channel>>,
    queue: VecDeque<MessageEntry>,
    pollers: Vec<SharedRef<Poller>>,
    wait_queue: WaitQueue,
}

pub struct Channel {
    mutable: SpinLock<Mutable>,
}

impl Channel {
    pub fn new() -> Result<(SharedRef<Channel>, SharedRef<Channel>), FtlError> {
        let ch0 = SharedRef::new(Channel {
            mutable: SpinLock::new(Mutable {
                peer: None,
                queue: VecDeque::new(),
                pollers: Vec::new(),
                wait_queue: WaitQueue::new(),
            }),
        });
        let ch1 = SharedRef::new(Channel {
            mutable: SpinLock::new(Mutable {
                peer: None,
                queue: VecDeque::new(),
                pollers: Vec::new(),
                wait_queue: WaitQueue::new(),
            }),
        });

        // TODO: Can we avoid this mutate-after-construct?
        ch0.mutable.lock().peer = Some(ch1.clone());
        ch1.mutable.lock().peer = Some(ch0.clone());

        Ok((ch0, ch1))
    }

    pub fn add_poller(&self, poller: SharedRef<Poller>) {
        let mut mutable = self.mutable.lock();

        if !mutable.queue.is_empty() {
            poller.set_ready(PollEvent::READABLE);
        }

        mutable.pollers.push(poller);
    }

    pub fn remove_poller(&self, poller: &SharedRef<Poller>) {
        let mut mutable = self.mutable.lock();
        mutable.pollers.retain(|p| !SharedRef::ptr_eq(p, poller));
    }

    pub fn send(&self, msginfo: MessageInfo, msgbuffer: UAddr) -> Result<(), FtlError> {
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

            // First loop: make sure moving handles won't fail and there are
            //             not too many ones.
            let mut handle_ids: InlinedVec<HandleId, MESSAGE_HANDLES_MAX_COUNT> = InlinedVec::new();
            for i in 0..num_handles {
                let handle_id = msgbuffer.read_from_user_at(
                    offset_of!(MessageBuffer, handles) + i * size_of::<HandleId>(),
                );
                handle_ids
                    .try_push(handle_id)
                    .map_err(|_| FtlError::TooManyHandles)?;

                if !our_handles.is_movable(handle_id) {
                    return Err(FtlError::HandleNotMovable);
                }
            }

            // Second loop: Remove handles from the current process.
            for i in 0..num_handles {
                // Note: Don't read the handle from the buffer again - user
                //       might have changed it (intentinally or not).
                let handle_id = handle_ids[i];

                // SAFETY: unwrap() won't panic because we've checked the handle
                //         is movable in the previous loop.
                let handle = our_handles.remove(handle_id).unwrap();

                // SAFETY: unwrap() won't panic because `handles` should have
                //         enough capacity up to MESSAGE_HANDLES_MAX_COUNT.
                moved_handles.try_push(handle).unwrap();
            }
        }

        // Copy message data into the kernel memory.
        let data_len = msginfo.data_len();
        let data = msgbuffer.read_from_user_to_vec::<u8>(offset_of!(MessageBuffer, data), data_len);

        let entry = MessageEntry {
            msginfo,
            data,
            handles: moved_handles,
        };

        let mutable = self.mutable.lock();
        let peer_ch = mutable.peer.as_ref().ok_or(FtlError::NoPeer)?;
        let mut peer_mutable = peer_ch.mutable.lock();
        peer_mutable.queue.push_back(entry);
        peer_mutable.wait_queue.wake_all();

        for poller in &peer_mutable.pollers {
            poller.set_ready(PollEvent::READABLE);
        }

        Ok(())
    }

    pub fn recv(
        self: &SharedRef<Channel>,
        msgbuffer: UAddr,
        blocking: bool,
        process: &SharedRef<Process>,
    ) -> Result<MessageInfo, FtlError> {
        let mut entry = {
            let mut mutable = self.mutable.lock();
            let entry = match mutable.queue.pop_front() {
                Some(entry) => entry,
                None => {
                    if blocking {
                        mutable.wait_queue.listen();
                        drop(mutable);

                        Thread::block_current(Continuation::ChannelRecv {
                            process: process.clone(),
                            channel: self.clone(),
                            msgbuffer,
                        });
                    }

                    return Err(FtlError::WouldBlock);
                }
            };

            if !mutable.queue.is_empty() {
                for poller in &mutable.pollers {
                    poller.set_ready(PollEvent::READABLE);
                }
            }

            entry
        };

        // Install handles into the current (receiver) process.
        let current_thread = current_thread();
        let mut handle_table = current_thread.process().handles().lock();
        for (i, any_handle) in entry.handles.drain(..).enumerate() {
            // TODO: Define the expected behavior when it fails to add a handle.
            let handle_id = handle_table.add(any_handle)?;
            msgbuffer.write_to_user_at(
                offset_of!(MessageBuffer, handles) + i * size_of::<HandleId>(),
                handle_id,
            );
        }

        // Copy message data into the buffer.
        let data_len = entry.msginfo.data_len();
        msgbuffer.write_to_user_at_slice(offset_of!(MessageBuffer, data), &entry.data[0..data_len]);

        Ok(entry.msginfo)
    }
}

impl fmt::Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Channel")
    }
}
