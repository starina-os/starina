use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use core::fmt;

use arrayvec::ArrayVec;
use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::message::MESSAGE_NUM_HANDLES_MAX;
use starina::message::MessageInfo;

use crate::cpuvar::current_thread;
use crate::handle::AnyHandle;
use crate::isolation::IsolationHeap;
use crate::process::Process;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;

/// A message queue entry.
struct MessageEntry {
    msginfo: MessageInfo,
    data: Vec<u8>,
    handles: ArrayVec<AnyHandle, MESSAGE_NUM_HANDLES_MAX>,
}

/// Channel object fields that are mutable.
struct Mutable {
    /// The peer channel. If it's `None`, the peer is not connected anymore
    /// and sending a message will fail.
    peer: Option<SharedRef<Channel>>,
    /// The received message queue.
    queue: VecDeque<MessageEntry>,
}

pub struct Channel {
    mutable: SpinLock<Mutable>,
}

impl Channel {
    /// Creates a channel pair.
    pub fn new() -> Result<(SharedRef<Channel>, SharedRef<Channel>), ErrorCode> {
        let ch0 = SharedRef::new(Channel {
            mutable: SpinLock::new(Mutable {
                peer: None,
                queue: VecDeque::new(),
            }),
        });
        let ch1 = SharedRef::new(Channel {
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
        msgbuffer: &IsolationHeap,
        handles: &IsolationHeap,
    ) -> Result<(), ErrorCode> {
        // Move handles.
        //
        // In this phase, since we don't know the receiver process, we don't
        // move to the desination process, but keep ownership of them (AnyHandle)
        // in the message entry.
        let num_handles = msginfo.num_handles();
        let mut moved_handles = ArrayVec::new();
        let current_thread = current_thread();
        let current_process = current_thread.process();
        if num_handles > 0 {
            // Note: Don't release this lock until we've moved all handles
            //       to guarantee that the second loop never fails.
            let mut our_handles = current_thread.process().handles().lock();

            // First loop: make sure moving handles won't fail and there are
            //             not too many ones.
            let mut handle_ids: ArrayVec<HandleId, MESSAGE_NUM_HANDLES_MAX> = ArrayVec::new();
            for i in 0..num_handles {
                let handle_id =
                    handles.read(current_process.isolation(), i * size_of::<HandleId>())?;

                // SAFETY: unwrap() won't panic because it should have enough
                //         capacity up to MESSAGE_HANDLES_MAX_COUNT.
                handle_ids.try_push(handle_id).unwrap();

                if !our_handles.is_movable(handle_id) {
                    return Err(ErrorCode::HandleNotMovable);
                }
            }

            // Second loop: Remove handles from the current process.
            for i in 0..num_handles {
                // Note: Don't read the handle from the buffer again - user
                //       might have changed it.
                let handle_id = handle_ids[i];

                // SAFETY: unwrap() won't panic because we've checked the handle
                //         is movable in the previous loop.
                let handle = our_handles.remove(handle_id).unwrap();

                // SAFETY: unwrap() won't panic because `handles` should have
                //         enough capacity up to MESSAGE_NUM_HANDLES_MAX.
                moved_handles.try_push(handle).unwrap();
            }
        }

        // Copy message data into the kernel memory.
        let data = msgbuffer.read_to_vec(current_process.isolation(), 0, msginfo.data_len())?;

        // Enqueue the message to the peer's queue.
        let mutable = self.mutable.lock();
        let peer_ch = mutable.peer.as_ref().ok_or(ErrorCode::NoPeer)?;
        let mut peer_mutable = peer_ch.mutable.lock();
        peer_mutable.queue.push_back(MessageEntry {
            msginfo,
            data,
            handles: moved_handles,
        });

        Ok(())
    }

    pub fn recv(
        self: &SharedRef<Channel>,
        msgbuffer: &mut IsolationHeap,
        handles: &mut IsolationHeap,
    ) -> Result<MessageInfo, ErrorCode> {
        let current_thread = current_thread();
        let current_process = current_thread.process();
        let mut entry = {
            let mut mutable = self.mutable.lock();
            let entry = match mutable.queue.pop_front() {
                Some(entry) => entry,
                None => {
                    return Err(ErrorCode::Empty);
                }
            };

            entry
        };

        // Install handles into the current (receiver) process.
        let mut handle_table = current_process.handles().lock();
        for (i, any_handle) in entry.handles.drain(..).enumerate() {
            // TODO: Define the expected behavior when it fails to add a handle.
            let handle_id = handle_table.insert(any_handle)?;
            handles.write(
                current_process.isolation(),
                i * size_of::<HandleId>(),
                handle_id,
            )?;
        }

        // Copy message data into the buffer.
        msgbuffer.write_bytes(
            current_process.isolation(),
            0,
            &entry.data[0..entry.msginfo.data_len()],
        )?;

        Ok(entry.msginfo)
    }
}

impl fmt::Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Channel")
    }
}
