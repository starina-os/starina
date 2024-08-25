use ftl_types::address::PAddr;
use ftl_types::address::VAddr;
use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::handle::HandleRights;
use ftl_types::interrupt::Irq;
use ftl_types::message::MessageBuffer;
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
use crate::ref_counted::SharedRef;
use crate::signal::Signal;

fn channel_create() -> Result<isize, FtlError> {
    let (ch1, ch2) = Channel::new()?;

    let current = current_thread();
    let mut handles = current.process().handles().lock();
    let handle0 = handles.add(Handle::new(ch1, HandleRights::NONE))?;
    let handle1 = handles.add(Handle::new(ch2, HandleRights::NONE))?;

    assert_eq!(handle0.as_isize() + 1, handle1.as_isize());
    Ok(handle0.as_isize())
}

fn channel_send(
    handle: HandleId,
    msginfo: MessageInfo,
    msgbuffer: &MessageBuffer,
) -> Result<(), FtlError> {
    let ch: Handle<Channel> = {
        current_thread()
            .process()
            .handles()
            .lock()
            .get_owned(handle)?
            .as_channel()?
            .clone()
    };

    ch.send(msginfo, msgbuffer)
}

fn channel_recv(handle: HandleId, msgbuffer: &mut MessageBuffer) -> Result<MessageInfo, FtlError> {
    let ch: Handle<Channel> = {
        current_thread()
            .process()
            .handles()
            .lock()
            .get_owned(handle)?
            .as_channel()?
            .clone()
    };

    ch.recv(msgbuffer)
}

fn folio_create(len: usize) -> Result<HandleId, FtlError> {
    let folio = Folio::alloc(len)?;
    let handle = Handle::new(SharedRef::new(folio), HandleRights::NONE);
    let handle_id = current_thread()
        .process()
        .handles()
        .lock()
        .add(AnyHandle::Folio(handle))?;

    Ok(handle_id)
}

fn folio_create_fixed(paddr: PAddr, len: usize) -> Result<HandleId, FtlError> {
    let folio = Folio::alloc_fixed(paddr, len)?;
    let handle = Handle::new(SharedRef::new(folio), HandleRights::NONE);
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
            .get_owned(handle)?
            .as_folio()?
            .clone()
    };

    Ok(folio.paddr())
}

fn poll_create() -> Result<HandleId, FtlError> {
    let poll = Poll::new();
    let handle = Handle::new(poll, HandleRights::NONE);

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
    let poll = handles.get_owned(poll_handle_id)?.as_poll()?;
    let object = handles.get_owned(target_handle_id)?;
    poll.add(object, target_handle_id, interests);
    Ok(())
}

fn poll_wait(handle_id: HandleId) -> Result<PollSyscallResult, FtlError> {
    let poll = {
        current_thread()
            .process()
            .handles()
            .lock()
            .get_owned(handle_id)?
            .as_poll()?
            .clone()
    };

    let (ev, ready_handle_id) = poll.wait()?;
    Ok(PollSyscallResult::new(ev, ready_handle_id))
}

fn signal_create() -> Result<HandleId, FtlError> {
    let signal = Signal::new()?;
    let handle = Handle::new(signal, HandleRights::NONE);
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
            .get_owned(handle_id)?
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
            .get_owned(handle_id)?
            .as_signal()?
            .clone()
    };

    signal.clear()
}

fn interrupt_create(irq: Irq) -> Result<HandleId, FtlError> {
    let interrupt = Interrupt::new(irq)?;
    let handle = Handle::new(interrupt, HandleRights::NONE);
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
            .get_owned(handle_id)?
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
        let vmspace = handles.get_owned(handle_id)?.as_vmspace()?.clone();
        let folio = handles.get_owned(folio)?.as_folio()?.clone();
        (vmspace, folio)
    };

    vmspace.map_anywhere(len, folio, prot)
}

pub fn syscall_entry(
    n: isize,
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    a5: isize,
) -> Result<isize, FtlError> {
    match n {
        _ if n == SyscallNumber::Print as isize => {
            let bytes = unsafe { core::slice::from_raw_parts(a0 as *const u8, a1 as usize) };
            let s = core::str::from_utf8(bytes).unwrap().trim_end();
            println!("{}", s);
            Ok(0)
        }
        _ if n == SyscallNumber::ChannelCreate as isize => channel_create(),
        _ if n == SyscallNumber::ChannelSend as isize => {
            let handle = HandleId::from_raw_isize_truncated(a0);
            let msginfo = MessageInfo::from_raw(a1);
            let msgbuffer = unsafe { &*(a2 as usize as *const MessageBuffer) };
            let err = channel_send(handle, msginfo, msgbuffer);
            if let Err(e) = err {
                return Err(e);
            }

            Ok(0)
        }
        _ if n == SyscallNumber::ChannelRecv as isize => {
            let handle = HandleId::from_raw_isize_truncated(a0);
            let msgbuffer = unsafe { &mut *(a1 as usize as *mut MessageBuffer) };
            let msginfo = channel_recv(handle, msgbuffer)?;
            Ok(msginfo.as_raw())
        }
        _ if n == SyscallNumber::FolioCreate as isize => {
            let handle_id = folio_create(a0 as usize)?;
            Ok(handle_id.as_isize())
        }
        _ if n == SyscallNumber::FolioCreateFixed as isize => {
            let paddr = PAddr::new(a0 as usize).ok_or(FtlError::InvalidArg)?;
            let len = a1 as usize;
            let handle_id = folio_create_fixed(paddr, len)?;
            Ok(handle_id.as_isize())
        }
        _ if n == SyscallNumber::FolioPAddr as isize => {
            let handle = HandleId::from_raw_isize_truncated(a0);
            let paddr = folio_paddr(handle)?;
            Ok(paddr.as_usize() as isize) // FIXME: guarantee casting to isize is OK
        }
        _ if n == SyscallNumber::PollCreate as isize => {
            let handle_id = poll_create()?;
            Ok(handle_id.as_isize())
        }
        _ if n == SyscallNumber::PollAdd as isize => {
            let poll_handle_id = HandleId::from_raw_isize_truncated(a0);
            let target_handle_id = HandleId::from_raw_isize_truncated(a1);
            let interests = PollEvent::from_raw(a2 as u8);
            if let Err(e) = poll_add(poll_handle_id, target_handle_id, interests) {
                return Err(e);
            }
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
        _ => {
            warn!(
                "unknown syscall: n={}, a0={}, a1={}, a2={}, a3={}, a4={}, a5={}",
                n, a0, a1, a2, a3, a4, a5
            );

            Err(FtlError::UnknownSyscall)
        }
    }
}
