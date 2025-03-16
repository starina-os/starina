use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use core::fmt;

use arrayvec::ArrayVec;
use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
use starina_types::message::MESSAGE_NUM_HANDLES_MAX;
use starina_types::message::MessageInfo;
use starina_types::poll::Readiness;

use crate::cpuvar::current_thread;
use crate::handle::AnyHandle;
use crate::handle::HandleTable;
use crate::handle::Handleable;
use crate::isolation::IsolationHeap;
use crate::isolation::IsolationHeapMut;
use crate::poll::Listener;
use crate::poll::ListenerSet;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;

pub const MESSAGE_QUEUE_MAX_LEN: usize = 128;

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
    listeners: ListenerSet,
}

impl Mutable {
    pub fn new() -> Self {
        Self {
            peer: None,
            queue: VecDeque::new(),
            listeners: ListenerSet::new(),
        }
    }
}

pub struct Channel {
    mutable: SpinLock<Mutable>,
}

impl Channel {
    /// Creates a channel pair.
    pub fn new() -> Result<(SharedRef<Channel>, SharedRef<Channel>), ErrorCode> {
        let ch0 = SharedRef::new(Channel {
            mutable: SpinLock::new(Mutable::new()),
        })?;
        let ch1 = SharedRef::new(Channel {
            mutable: SpinLock::new(Mutable::new()),
        })?;

        // TODO: Can we avoid this mutate-after-construct?
        ch0.mutable.lock().peer = Some(ch1.clone());
        ch1.mutable.lock().peer = Some(ch0.clone());

        Ok((ch0, ch1))
    }

    pub fn send(
        &self,
        handle_table: &mut HandleTable,
        msginfo: MessageInfo,
        msgbuffer: &IsolationHeap,
        handles: &IsolationHeap,
    ) -> Result<(), ErrorCode> {
        let current_thread = current_thread();
        let isolation = current_thread.process().isolation();

        // Copy message data into the kernel memory. Do this before locking
        // the peer channel for better performance. This memory copy might
        // take a long time.
        let data = msgbuffer.read_to_vec(isolation, 0, msginfo.data_len())?;

        // Enqueue the message to the peer's queue.
        let mutable = self.mutable.lock();
        let peer_ch = mutable.peer.as_ref().ok_or(ErrorCode::NoPeer)?;
        let mut peer_mutable = peer_ch.mutable.lock();

        // Check if the peer's queue is full.
        if peer_mutable.queue.len() >= MESSAGE_QUEUE_MAX_LEN {
            return Err(ErrorCode::Full);
        }

        // Allocate space for the message in the peer's queue so that
        // `VecDeque::push_back` won't panic.
        if peer_mutable.queue.try_reserve_exact(1).is_err() {
            return Err(ErrorCode::OutOfMemory);
        }

        // Move handles.
        //
        // In this phase, since we don't know the receiver process, we don't
        // move to the desination process, but keep ownership of them (AnyHandle)
        // in the message entry.
        let num_handles = msginfo.num_handles();
        let mut moved_handles = ArrayVec::new();
        if num_handles > 0 {
            // Note: Don't release this lock until we've moved all handles
            //       to guarantee that the second loop never fails.

            // First loop: make sure moving handles won't fail and there are
            //             not too many ones.
            let mut handle_ids: ArrayVec<HandleId, MESSAGE_NUM_HANDLES_MAX> = ArrayVec::new();
            for i in 0..num_handles {
                let handle_id = handles.read(isolation, i * size_of::<HandleId>())?;

                // SAFETY: unwrap() won't panic because it should have enough
                //         capacity up to MESSAGE_HANDLES_MAX_COUNT.
                handle_ids.try_push(handle_id).unwrap();

                if !handle_table.is_movable(handle_id) {
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
                let handle = handle_table.take(handle_id).unwrap();

                // SAFETY: unwrap() won't panic because `handles` should have
                //         enough capacity up to MESSAGE_NUM_HANDLES_MAX.
                moved_handles.try_push(handle).unwrap();
            }
        }

        // The message is ready to be sent. Enqueue it.
        peer_mutable.queue.push_back(MessageEntry {
            msginfo,
            data,
            handles: moved_handles,
        });

        // So the peer has at least one message to read. Wake up a listener if any.
        peer_mutable.listeners.notify_all(Readiness::READABLE);

        Ok(())
    }

    pub fn recv(
        self: &SharedRef<Channel>,
        handle_table: &mut HandleTable,
        msgbuffer: &mut IsolationHeapMut,
        handles: &mut IsolationHeapMut,
    ) -> Result<MessageInfo, ErrorCode> {
        let current_thread = current_thread();
        let isolation = current_thread.process().isolation();
        let mut entry = {
            let mut mutable = self.mutable.lock();
            let entry = match mutable.queue.pop_front() {
                Some(entry) => entry,
                None => {
                    // Check if the peer is still connected only if the queue is
                    // empty. This is to allow the peer to close the channel before
                    // waiting for us to read all messages.
                    return if mutable.peer.is_some() {
                        // We have no message to read *for now*. The peer might
                        // send a message later.
                        Err(ErrorCode::Empty)
                    } else {
                        // We'll never receive a message anymore. Tell the caller
                        // that you're done.
                        Err(ErrorCode::NoPeer)
                    };
                }
            };

            if !mutable.queue.is_empty() {
                // There are more messages in the queue. Mark this channel as
                // still readable.
                mutable.listeners.notify_all(Readiness::READABLE);
            }

            if let Some(peer) = &mutable.peer {
                // The peer is still connected. Notify the peer channel's
                // listeners that we're ready to receive at least one message.
                peer.mutable
                    .lock()
                    .listeners
                    .notify_all(Readiness::WRITABLE);
            }

            entry
        };

        // Install handles into the current (receiver) process.
        for (i, any_handle) in entry.handles.drain(..).enumerate() {
            // TODO: Define the expected behavior when it fails to add a handle.
            let handle_id = handle_table.insert(any_handle)?;
            handles.write(isolation, i * size_of::<HandleId>(), handle_id)?;
        }

        // Copy message data into the buffer.
        msgbuffer.write_bytes(isolation, 0, &entry.data[0..entry.msginfo.data_len()])?;

        Ok(entry.msginfo)
    }
}

impl Handleable for Channel {
    fn close(&self) {
        let mutable = self.mutable.lock();
        if let Some(peer) = &mutable.peer {
            peer.mutable.lock().peer = None;
        }
    }

    fn add_listener(&self, listener: Listener) -> Result<(), ErrorCode> {
        self.mutable.lock().listeners.add_listener(listener)?;
        Ok(())
    }

    fn remove_listener(&self, poll: &crate::poll::Poll) -> Result<(), ErrorCode> {
        self.mutable.lock().listeners.remove_listener(poll);
        Ok(())
    }

    fn readiness(&self) -> Result<Readiness, ErrorCode> {
        let mut readiness = Readiness::new();
        let mutable = self.mutable.lock();
        if !mutable.queue.is_empty() {
            readiness |= Readiness::READABLE;
        }

        if let Some(peer) = mutable.peer.as_ref() {
            let peer_mutable = peer.mutable.lock();
            if peer_mutable.queue.len() < MESSAGE_QUEUE_MAX_LEN {
                readiness |= Readiness::WRITABLE;
            }
        }

        Ok(readiness)
    }
}

impl fmt::Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Channel")
    }
}
