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

use crate::arch;
use crate::channel::Channel;
use crate::cpuvar::current_thread;
use crate::folio::Folio;
use crate::handle::AnyHandle;
use crate::handle::Handle;
use crate::interrupt::Interrupt;
use crate::memory::AllocPagesError;
use crate::poll::Poll;
use crate::ref_counted::SharedRef;
use crate::signal::Signal;

fn channel_create() -> Result<isize, FtlError> {
    todo!();

    // TODO:
    // assert_eq!(handle0 + 1, handle1);
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
    let folio = match Folio::alloc(len) {
        Ok(folio) => folio,
        Err(AllocPagesError::InvalidLayout(_err)) => {
            return Err(FtlError::InvalidArg);
        }
    };

    let handle = Handle::new(SharedRef::new(folio), HandleRights::NONE);
    let handle_id = current_thread()
        .process()
        .handles()
        .lock()
        .add(AnyHandle::Folio(handle))?;

    Ok(handle_id)
}

fn folio_create_mmio(paddr: PAddr, len: usize) -> Result<HandleId, FtlError> {
    let folio = match Folio::alloc_mmio(paddr, len) {
        Ok(folio) => folio,
        Err(AllocPagesError::InvalidLayout(_err)) => {
            return Err(FtlError::InvalidArg);
        }
    };

    let handle = Handle::new(SharedRef::new(folio), HandleRights::NONE);
    let handle_id = current_thread()
        .process()
        .handles()
        .lock()
        .add(AnyHandle::Folio(handle))?;

    Ok(handle_id)
}

fn folio_vaddr(handle: HandleId) -> Result<VAddr, FtlError> {
    let folio: Handle<Folio> = {
        current_thread()
            .process()
            .handles()
            .lock()
            .get_owned(handle)?
            .as_folio()?
            .clone()
    };

    folio.vaddr()
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

    folio.paddr()
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
                println!("channel_send failed: {:?}", e);
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
        _ if n == SyscallNumber::FolioCreateMmio as isize => {
            let paddr = PAddr::new(a0 as usize).ok_or(FtlError::InvalidArg)?;
            let len = a1 as usize;
            let handle_id = folio_create_mmio(paddr, len)?;
            Ok(handle_id.as_isize())
        }
        _ if n == SyscallNumber::FolioVAddr as isize => {
            let handle = HandleId::from_raw_isize_truncated(a0);
            let vaddr = folio_vaddr(handle)?;
            Ok(vaddr.as_usize() as isize) // FIXME: guarantee casting to isize is OK
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
                println!("poll_add failed: {:?}", e);
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
        _ => {
            println!(
                "unknown syscall: n={}, a0={}, a1={}, a2={}, a3={}, a4={}, a5={}",
                n, a0, a1, a2, a3, a4, a5
            );

            Err(FtlError::UnknownSyscall)
        }
    }
}
