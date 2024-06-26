use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::handle::HandleRights;
use ftl_types::message::MessageBuffer;
use ftl_types::message::MessageInfo;
use ftl_types::syscall::SyscallNumber;
use ftl_types::syscall::VsyscallPage;

use crate::buffer::Buffer;
use crate::channel::Channel;
use crate::cpuvar::current_thread;
use crate::handle::AnyHandle;
use crate::handle::Handle;
use crate::memory::AllocPagesError;
use crate::ref_counted::SharedRef;

pub const VSYSCALL_PAGE: VsyscallPage = VsyscallPage {
    entry: syscall_entry,
};

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

fn buffer_create(len: usize) -> Result<HandleId, FtlError> {
    let buffer = match Buffer::alloc(len) {
        Ok(buffer) => buffer,
        Err(AllocPagesError::InvalidLayout(error)) => {
            return Err(FtlError::InvalidArg);
        }
    };

    let handle = Handle::new(SharedRef::new(buffer), HandleRights::NONE);
    let handle_id = current_thread()
        .process()
        .handles()
        .lock()
        .add(AnyHandle::Buffer(handle))?;

    Ok(handle_id)
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
            println!("[print] {}", s);
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
        _ if n == SyscallNumber::BufferCreate as isize => {
            let handle_id = buffer_create(a0 as usize)?;
            Ok(handle_id.as_isize())
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
