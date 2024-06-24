use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::message::MessageInfo;
use ftl_types::message::MESSAGE_DATA_MAX_LEN;
use ftl_types::message::MESSAGE_HANDLES_MAX_COUNT;
use ftl_types::syscall::SyscallNumber;
use ftl_types::syscall::VsyscallPage;

use crate::channel::Channel;
use crate::cpuvar::current_thread;
use crate::handle::Handle;

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
    buf: &[u8; MESSAGE_DATA_MAX_LEN],
    handles: &[HandleId; MESSAGE_HANDLES_MAX_COUNT],
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

    ch.send(msginfo, buf, handles)
}

fn channel_recv(
    handle: HandleId,
    buf: &mut [u8; MESSAGE_DATA_MAX_LEN],
    handles: &mut [HandleId; MESSAGE_HANDLES_MAX_COUNT],
) -> Result<MessageInfo, FtlError> {
    let ch: Handle<Channel> = {
        current_thread()
            .process()
            .handles()
            .lock()
            .get_owned(handle)?
            .as_channel()?
            .clone()
    };

    ch.recv(buf, handles)
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
            let buf = unsafe { &*(a2 as usize as *const [u8; MESSAGE_DATA_MAX_LEN]) };
            let handles = unsafe { &*(a3 as usize as *const [HandleId; MESSAGE_HANDLES_MAX_COUNT]) };
            let err = channel_send(handle, msginfo, buf, handles);
            if let Err(e) = err {
                println!("channel_send failed: {:?}", e);
                return Err(e);
            }

            Ok(0)
        }
        _ if n == SyscallNumber::ChannelRecv as isize => {
            let handle = HandleId::from_raw_isize_truncated(a0);
            let buf = unsafe { &mut *(a1 as usize as *mut [u8; MESSAGE_DATA_MAX_LEN]) };
            let handles = unsafe { &mut *(a2 as usize as *mut [HandleId; MESSAGE_HANDLES_MAX_COUNT]) };
            let msginfo = channel_recv(handle, buf, handles)?;
            Ok(msginfo.as_raw())
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
