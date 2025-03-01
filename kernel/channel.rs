use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use core::fmt;

use arrayvec::ArrayVec;
use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::message::MESSAGE_NUM_HANDLES_MAX;
use starina::message::MessageInfo;
use starina::poll::Readiness;

use crate::cpuvar::current_thread;
use crate::handle::AnyHandle;
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
        });
        let ch1 = SharedRef::new(Channel {
            mutable: SpinLock::new(Mutable::new()),
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
        let current_thread = current_thread();
        let current_process = current_thread.process();

        // Copy message data into the kernel memory. Do this before locking
        // the peer channel for better performance. This memory copy might
        // take a long time.
        let data = msgbuffer.read_to_vec(current_process.isolation(), 0, msginfo.data_len())?;

        // Enqueue the message to the peer's queue.
        let mutable = self.mutable.lock();
        let peer_ch = mutable.peer.as_ref().ok_or(ErrorCode::NoPeer)?;
        let mut peer_mutable = peer_ch.mutable.lock();

        // Check if the peer's queue is full.
        if peer_mutable.queue.len() >= MESSAGE_QUEUE_MAX_LEN {
            return Err(ErrorCode::Full);
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

        // The message is ready to be sent. Enqueue it.
        peer_mutable.queue.push_back(MessageEntry {
            msginfo,
            data,
            handles: moved_handles,
        });

        // So the peer has at least one message to read. Wake up a listener if any.
        peer_mutable.listeners.mark_ready(Readiness::READABLE);

        Ok(())
    }

    pub fn recv(
        self: &SharedRef<Channel>,
        msgbuffer: &mut IsolationHeapMut,
        handles: &mut IsolationHeapMut,
    ) -> Result<MessageInfo, ErrorCode> {
        let current_thread = current_thread();
        let current_process = current_thread.process();
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
                mutable.listeners.mark_ready(Readiness::READABLE);
            }

            if let Some(peer) = &mutable.peer {
                // The peer is still connected. Notify the peer channel's
                // listeners that we're ready to receive at least one message.
                peer.mutable
                    .lock()
                    .listeners
                    .mark_ready(Readiness::WRITABLE);
            }

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

impl Handleable for Channel {
    fn add_listener(&self, listener: SharedRef<Listener>) {
        self.mutable.lock().listeners.add_listener(listener);
    }

    fn readiness(&self) -> Readiness {
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

        readiness
    }
}

impl fmt::Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Channel")
    }
}

#[cfg(test)]
mod tests {
    use core::cell::RefCell;

    use super::*;
    use crate::arch::set_cpuvar;
    use crate::cpuvar::CpuId;
    use crate::isolation::IsolationHeap;
    use crate::isolation::IsolationHeapMut;
    use crate::thread::Thread;

    #[test]
    fn test_send_and_recv() {
        let idle_thread = Thread::new_idle();
        let thread = Thread::new_inkernel(0, 0);
        set_cpuvar(Box::leak(Box::new(crate::cpuvar::CpuVar {
            arch: crate::arch::CpuVar::new(&thread),
            cpu_id: CpuId::new(0),
            current_thread: RefCell::new(thread.clone()),
            idle_thread,
        })));

        let (ch1, ch2) = Channel::new().unwrap();
        let send_buf = b"BEEP BEEP BEEP\0\0";
        let send_heap = IsolationHeap::InKernel {
            ptr: send_buf.as_ptr(),
            len: send_buf.len(),
        };
        let handles_heap = IsolationHeap::InKernel {
            ptr: core::ptr::null_mut(),
            len: 0,
        };
        info!("Sending message...");
        ch1.send(
            MessageInfo::new(0, send_buf.len().try_into().unwrap(), 0),
            &send_heap,
            &handles_heap,
        )
        .unwrap();

        let mut recv_buf = [0u8; 16];
        let mut recv_heap = IsolationHeapMut::InKernel {
            ptr: recv_buf.as_mut_ptr(),
            len: recv_buf.len(),
        };
        let mut handles_heap = IsolationHeapMut::InKernel {
            ptr: core::ptr::null_mut(),
            len: 0,
        };
        ch2.recv(&mut recv_heap, &mut handles_heap).unwrap();

        assert_eq!(&recv_buf, send_buf);
    }
}
