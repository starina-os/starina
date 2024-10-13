//! System call implementation.
//!
//! # System call handler cannot block!
//!
//! Due to the single-stack kernel design, the system call handler cannot
//! block. Instead, when it needs to wait for an event, it should save the
//! state into [`Continuation`](crate::thread::Continuation), switch to another
//! thread, and retry later.
use ftl_types::address::PAddr;
use ftl_types::address::VAddr;
use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::handle::HandleRights;
use ftl_types::interrupt::Irq;
use ftl_types::message::MessageInfo;
use ftl_types::poll::PollEvent;
use ftl_types::poll::PollSyscallResult;
use ftl_types::signal::SignalBits;
use ftl_types::syscall::SyscallNumber;
use ftl_types::vmspace::PageProtect;

use crate::channel::Channel;
use crate::cpuvar::current_thread;
use crate::folio::Folio;
use crate::handle::AnyHandle;
use crate::handle::Handle;
use crate::interrupt::Interrupt;
use crate::poll::Poll;
use crate::process::Process;
use crate::refcount::SharedRef;
use crate::signal::Signal;
use crate::uaddr::UAddr;

fn console_write(uaddr: UAddr, len: usize) {
    // TODO: Avoid copying the entire string into kernel.
    let bytes = uaddr.read_from_user_to_vec(0, len);
    let s = core::str::from_utf8(&bytes).unwrap().trim_end();
    println!("{}", s);
}

fn channel_create() -> Result<isize, FtlError> {
    let (ch1, ch2) = Channel::new()?;

    let current = current_thread();
    let mut handles = current.process().handles().lock();
    let handle0 = handles.add(Handle::new(ch1, HandleRights::ALL))?;
    let handle1 = handles.add(Handle::new(ch2, HandleRights::ALL))?;

    assert_eq!(handle0.as_isize() + 1, handle1.as_isize());
    Ok(handle0.as_isize())
}

fn channel_send(handle: HandleId, msginfo: MessageInfo, msgbuffer: UAddr) -> Result<(), FtlError> {
    let ch: Handle<Channel> = {
        current_thread()
            .process()
            .handles()
            .lock()
            .get_owned(handle, HandleRights::WRITE)?
            .as_channel()?
            .clone()
    };

    ch.send(msginfo, msgbuffer)
}

fn channel_recv(handle: HandleId, msgbuffer: UAddr) -> Result<MessageInfo, FtlError> {
    let (process, ch): (SharedRef<Process>, Handle<Channel>) = {
        let process = current_thread().process().clone();

        let ch = process
            .handles()
            .lock()
            .get_owned(handle, HandleRights::READ)?
            .as_channel()?
            .clone();

        (process, ch)
    };

    let ch_ref = Handle::into_shared_ref(ch);
    ch_ref.recv(msgbuffer, true, &process)
}

fn channel_try_recv(handle: HandleId, msgbuffer: UAddr) -> Result<MessageInfo, FtlError> {
    let (process, ch): (SharedRef<Process>, Handle<Channel>) = {
        let process = current_thread().process().clone();

        let ch = process
            .handles()
            .lock()
            .get_owned(handle, HandleRights::READ)?
            .as_channel()?
            .clone();

        (process, ch)
    };

    let ch_ref = Handle::into_shared_ref(ch);
    ch_ref.recv(msgbuffer, false, &process)
}

fn channel_call(
    handle: HandleId,
    msginfo: MessageInfo,
    msgbuffer: UAddr,
) -> Result<MessageInfo, FtlError> {
    let (process, ch): (SharedRef<Process>, Handle<Channel>) = {
        let process = current_thread().process().clone();

        let ch = process
            .handles()
            .lock()
            .get_owned(handle, HandleRights::READ)?
            .as_channel()?
            .clone();

        (process, ch)
    };

    let ch_ref = Handle::into_shared_ref(ch);
    ch_ref.call(msginfo, msgbuffer, true, &process)
}

fn folio_create(len: usize) -> Result<HandleId, FtlError> {
    let folio = Folio::alloc(len)?;
    let handle = Handle::new(SharedRef::new(folio), HandleRights::ALL);
    let handle_id = current_thread()
        .process()
        .handles()
        .lock()
        .add(AnyHandle::Folio(handle))?;

    Ok(handle_id)
}

fn folio_create_fixed(paddr: PAddr, len: usize) -> Result<HandleId, FtlError> {
    let folio = Folio::alloc_fixed(paddr, len)?;
    let handle = Handle::new(SharedRef::new(folio), HandleRights::ALL);
    let handle_id = current_thread()
        .process()
        .handles()
        .lock()
        .add(AnyHandle::Folio(handle))?;

    Ok(handle_id)
}

fn folio_paddr(handle: HandleId) -> Result<PAddr, FtlError> {
    let folio: Handle<Folio> = {
        current_thread()
            .process()
            .handles()
            .lock()
            .get_owned(handle, HandleRights::DRIVER)?
            .as_folio()?
            .clone()
    };

    Ok(folio.paddr())
}

fn poll_create() -> Result<HandleId, FtlError> {
    let poll = Poll::new();
    let handle = Handle::new(poll, HandleRights::ALL);

    let handle_id = current_thread()
        .process()
        .handles()
        .lock()
        .add(AnyHandle::Poll(handle))?;

    Ok(handle_id)
}

fn poll_add(
    poll_handle_id: HandleId,
    target_handle_id: HandleId,
    interests: PollEvent,
) -> Result<(), FtlError> {
    let current_thread = current_thread();
    let handles = current_thread.process().handles().lock();
    let poll = handles
        .get_owned(poll_handle_id, HandleRights::WRITE)?
        .as_poll()?;
    let object = handles.get_owned(target_handle_id, HandleRights::POLL)?;
    poll.add(object, target_handle_id, interests)?;
    Ok(())
}

fn poll_remove(poll_handle_id: HandleId, target_handle_id: HandleId) -> Result<(), FtlError> {
    let current_thread = current_thread();
    let handles = current_thread.process().handles().lock();
    let poll = handles
        .get_owned(poll_handle_id, HandleRights::WRITE)?
        .as_poll()?;

    poll.remove(target_handle_id)?;
    Ok(())
}

fn poll_wait(handle_id: HandleId) -> Result<PollSyscallResult, FtlError> {
    let poll = {
        current_thread()
            .process()
            .handles()
            .lock()
            .get_owned(handle_id, HandleRights::READ | HandleRights::READ)?
            .as_poll()?
            .clone()
    };

    let (ev, ready_handle_id) = Handle::into_shared_ref(poll).wait(true)?;
    Ok(PollSyscallResult::new(ev, ready_handle_id))
}

fn signal_create() -> Result<HandleId, FtlError> {
    let signal = Signal::new()?;
    let handle = Handle::new(signal, HandleRights::ALL);
    let handle_id = current_thread()
        .process()
        .handles()
        .lock()
        .add(AnyHandle::Signal(handle))?;

    Ok(handle_id)
}

fn signal_update(handle_id: HandleId, value: SignalBits) -> Result<(), FtlError> {
    let signal: Handle<Signal> = {
        current_thread()
            .process()
            .handles()
            .lock()
            .get_owned(handle_id, HandleRights::WRITE)?
            .as_signal()?
            .clone()
    };

    signal.update(value)
}

fn signal_clear(handle_id: HandleId) -> Result<SignalBits, FtlError> {
    let signal: Handle<Signal> = {
        current_thread()
            .process()
            .handles()
            .lock()
            .get_owned(handle_id, HandleRights::WRITE)?
            .as_signal()?
            .clone()
    };

    signal.clear()
}

fn interrupt_create(irq: Irq) -> Result<HandleId, FtlError> {
    let interrupt = Interrupt::new(irq)?;
    let handle = Handle::new(interrupt, HandleRights::ALL);
    let handle_id = current_thread()
        .process()
        .handles()
        .lock()
        .add(AnyHandle::Interrupt(handle))?;

    Ok(handle_id)
}

fn interrupt_ack(handle_id: HandleId) -> Result<(), FtlError> {
    let interrupt: Handle<Interrupt> = {
        current_thread()
            .process()
            .handles()
            .lock()
            .get_owned(handle_id, HandleRights::WRITE)?
            .as_interrupt()?
            .clone()
    };

    interrupt.ack()
}

fn handle_close(handle_id: HandleId) -> Result<(), FtlError> {
    current_thread()
        .process()
        .handles()
        .lock()
        .remove(handle_id)?;

    Ok(())
}

fn vmspace_map(
    handle_id: HandleId,
    len: usize,
    folio: HandleId,
    prot: PageProtect,
) -> Result<VAddr, FtlError> {
    let (vmspace, folio) = {
        let current = current_thread();
        let handles = current.process().handles().lock();
        let vmspace = handles
            .get_owned(handle_id, HandleRights::MAP)?
            .as_vmspace()?
            .clone();
        let folio = handles
            .get_owned(folio, HandleRights::MAP)?
            .as_folio()?
            .clone();
        (vmspace, folio)
    };

    vmspace.map_anywhere(len, folio, prot)
}

fn process_exit() -> ! {
    Process::exit_current()
}

fn handle_syscall(
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    n: isize,
) -> Result<isize, FtlError> {
    match n {
        _ if n == SyscallNumber::ConsoleWrite as isize => {
            console_write(UAddr::new(a0 as usize), a1 as usize);
            Ok(0)
        }
        _ if n == SyscallNumber::ChannelCreate as isize => {
            let first_handle = channel_create()?;
            Ok(first_handle)
        }
        _ if n == SyscallNumber::ChannelSend as isize => {
            let handle = HandleId::from_raw_isize_truncated(a0);
            let msginfo = MessageInfo::from_raw(a1);
            let msgbuffer = UAddr::new(a2 as usize);
            channel_send(handle, msginfo, msgbuffer)?;
            Ok(0)
        }
        _ if n == SyscallNumber::ChannelRecv as isize => {
            let handle = HandleId::from_raw_isize_truncated(a0);
            let msgbuffer = UAddr::new(a1 as usize);
            let msginfo = channel_recv(handle, msgbuffer)?;
            Ok(msginfo.as_raw())
        }
        _ if n == SyscallNumber::ChannelTryRecv as isize => {
            let handle = HandleId::from_raw_isize_truncated(a0);
            let msgbuffer = UAddr::new(a1 as usize);
            let msginfo = channel_try_recv(handle, msgbuffer)?;
            Ok(msginfo.as_raw())
        }
        _ if n == SyscallNumber::ChannelCall as isize => {
            let handle = HandleId::from_raw_isize_truncated(a0);
            let request_msginfo = MessageInfo::from_raw(a1);
            let msgbuffer = UAddr::new(a2 as usize);
            let reply_msginfo = channel_call(handle, request_msginfo, msgbuffer)?;
            Ok(reply_msginfo.as_raw())
        }
        _ if n == SyscallNumber::FolioCreate as isize => {
            let handle_id = folio_create(a0 as usize)?;
            Ok(handle_id.as_isize())
        }
        _ if n == SyscallNumber::FolioCreateFixed as isize => {
            let paddr = PAddr::new(a0 as usize);
            let len = a1 as usize;
            let handle_id = folio_create_fixed(paddr, len)?;
            Ok(handle_id.as_isize())
        }
        _ if n == SyscallNumber::FolioPAddr as isize => {
            let handle = HandleId::from_raw_isize_truncated(a0);
            let paddr = folio_paddr(handle)?;

            // Try to convert PAddr to isize. We can't cast PAddr to isize directly
            // because negative values are considered as error codes. This hack should
            // be fine until your computer has more than 2^63 bytes of memory.
            let paddr_isize: isize = match paddr.as_usize().try_into() {
                Ok(value) => value,
                Err(_) => return Err(FtlError::TooLargePAddr),
            };

            Ok(paddr_isize)
        }
        _ if n == SyscallNumber::PollCreate as isize => {
            let handle_id = poll_create()?;
            Ok(handle_id.as_isize())
        }
        _ if n == SyscallNumber::PollAdd as isize => {
            let poll_handle_id = HandleId::from_raw_isize_truncated(a0);
            let target_handle_id = HandleId::from_raw_isize_truncated(a1);
            let interests = PollEvent::from_raw(a2 as u8);
            poll_add(poll_handle_id, target_handle_id, interests)?;
            Ok(0)
        }
        _ if n == SyscallNumber::PollRemove as isize => {
            let poll_handle_id = HandleId::from_raw_isize_truncated(a0);
            let target_handle_id = HandleId::from_raw_isize_truncated(a1);
            poll_remove(poll_handle_id, target_handle_id)?;
            Ok(0)
        }
        _ if n == SyscallNumber::PollWait as isize => {
            let handle_id = HandleId::from_raw_isize_truncated(a0);
            let result = poll_wait(handle_id)?;
            Ok(result.as_raw())
        }
        _ if n == SyscallNumber::SignalCreate as isize => {
            let handle_id = signal_create()?;
            Ok(handle_id.as_isize())
        }
        _ if n == SyscallNumber::SignalUpdate as isize => {
            let handle_id = HandleId::from_raw_isize_truncated(a0);
            let value = SignalBits::from_raw(a1 as i32);
            if let Err(e) = signal_update(handle_id, value) {
                println!("signal_update failed: {:?}", e);
                return Err(e);
            }
            Ok(0)
        }
        _ if n == SyscallNumber::SignalClear as isize => {
            let handle_id = HandleId::from_raw_isize_truncated(a0);
            let value = signal_clear(handle_id)?;
            Ok(value.as_i32() as isize)
        }
        _ if n == SyscallNumber::InterruptCreate as isize => {
            let irq = Irq::from_raw(a0 as usize);
            let handle_id = interrupt_create(irq)?;
            Ok(handle_id.as_isize())
        }
        _ if n == SyscallNumber::InterruptAck as isize => {
            let handle_id = HandleId::from_raw_isize_truncated(a0);
            interrupt_ack(handle_id)?;
            Ok(0)
        }
        _ if n == SyscallNumber::HandleClose as isize => {
            let handle_id = HandleId::from_raw_isize_truncated(a0);
            handle_close(handle_id)?;
            Ok(0)
        }
        _ if n == SyscallNumber::VmSpaceMap as isize => {
            let handle_id = HandleId::from_raw_isize_truncated(a0);
            let len = a1 as usize;
            let folio = HandleId::from_raw_isize_truncated(a2);
            let prot = PageProtect::from_raw(a3 as u8);
            let vaddr = vmspace_map(handle_id, len, folio, prot)?;
            Ok(vaddr.as_usize() as isize)
        }
        _ if n == SyscallNumber::ProcessExit as isize => {
            process_exit();
        }
        _ => {
            warn!(
                "unknown syscall: n={}, a0={}, a1={}, a2={}, a3={}, a4={}",
                n, a0, a1, a2, a3, a4,
            );

            Err(FtlError::UnknownSyscall)
        }
    }
}

/// The system call handler. Handles system calls with the given arguments,
/// and returns its return value.
///
/// `arch` layer should call this function when a system call is made.
pub fn syscall_handler(a0: isize, a1: isize, a2: isize, a3: isize, a4: isize, n: isize) -> isize {
    match handle_syscall(a0, a1, a2, a3, a4, n) {
        Ok(isize) => isize,
        Err(err) => -(err as isize),
    }
}
