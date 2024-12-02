#![no_std]
use core::convert::TryInto;

#[repr(isize)]
pub enum SyscallNumber {
    ChannelSend = 1,
}

#[repr(isize)]
pub enum FtlError {
    NoHandle = 1,
}

// pub type VsyscallEntry = extern "C" fn(isize, isize, isize, isize, isize, isize) -> isize;
// static mut VSYSCALL_ENTRY: *const VsyscallEntry = core::ptr::null();

pub fn syscall(
    n: SyscallNumber,
    rdi: isize,
    rsi: isize,
    rdx: isize,
    rcx: isize,
    r8: isize,
) -> Result<isize, FtlError> {
    use core::arch::asm;

    // let mut rax: isize = unsafe { VSYSCALL_ENTRY as isize };
    let mut rax: isize;
    unsafe {
        asm!(
            "call gs:[0]",
            in("rdi") rdi,
            in("rsi") rsi,
            in("rdx") rdx,
            in("rcx") rcx,
            in("r8") r8,
            in("r9") n as isize,
            out("rax") rax,
            options(nostack),
            clobber_abi("C"),
        );
    }

    if rax < 0 {
        unsafe { Err(core::mem::transmute::<isize, FtlError>(rax)) }
    } else {
        Ok(rax)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HandleId(i32);

#[derive(PartialEq, Eq)]
#[repr(transparent)]
pub struct OwnedHandle(HandleId);

#[derive(Debug)]
#[repr(transparent)]
pub struct MessageInfo(u32);

impl MessageInfo {
    const fn new(id: u32, len: u16, num_handles: u8) -> MessageInfo {
        debug_assert!(id < 1 << 16);
        debug_assert!(len < 1 << 14);
        debug_assert!(num_handles < 1 << 2);
        MessageInfo(id << 16 | (num_handles as u32) << 14 | (len as u32))
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Bytes<const CAP: usize> {
    len: u16,
    data: [u8; CAP],
}

impl<const CAP: usize> Bytes<CAP> {
    fn new(data: &[u8]) -> Bytes<CAP> {
        let copy_len = data.len();
        debug_assert!(copy_len < CAP);
        let mut buf =  core::mem::MaybeUninit::<[u8; CAP]>::uninit();
        let dst = unsafe {
            core::ptr::copy_nonoverlapping(data.as_ptr(), buf.as_mut_ptr() as *mut u8, copy_len);
            buf.assume_init()
        };

        Bytes {
            len: copy_len as u16,
            data: dst,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooManyBytesError;

impl<const CAP: usize> TryInto<Bytes<CAP>> for &[u8] {
    type Error = TooManyBytesError;

    fn try_into(self) -> Result<Bytes<CAP>, Self::Error> {
        if self.len() <= CAP {
            Ok(Bytes::new(self))
        } else {
            Err(TooManyBytesError)
        }
    }
}

pub trait MessageBody {
    fn msginfo(&self) -> MessageInfo;
}

#[repr(C)]
#[derive(Debug)]
pub struct Ping {
    value: u32,
}

impl MessageBody for Ping {
    fn msginfo(&self) -> MessageInfo {
        MessageInfo::new(0, core::mem::size_of::<Ping>() as u16, 0)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct TcpWrite {
    pub data: Bytes<512>,
}

impl MessageBody for TcpWrite {
    fn msginfo(&self) -> MessageInfo {
        MessageInfo::new(0, core::mem::size_of::<TcpWrite>() as u16 + self.data.len, 0)
    }
}

#[derive(Debug)]
pub enum Message {
    Ping(Ping),
    TcpWrite(TcpWrite),
}

pub struct MessageBuffer {
    pub data: [u8; 2047],
}

impl MessageBuffer {
    pub fn new() -> MessageBuffer {
        MessageBuffer { data: [0; 2047] }
    }
}

// extern "C" {
//     fn sys_channel_send(handle: HandleId, info: MessageInfo, m: *const u8) -> Result<(), FtlError>;
// }
fn sys_channel_send(handle: HandleId, info: MessageInfo, m: *const u8) -> Result<(), FtlError> {
    syscall(
        SyscallNumber::ChannelSend,
        handle.0 as isize,
        info.0 as isize,
        m as isize,
        0,
        0,
    )?;

    Ok(())
}

pub struct Channel {
    handle: OwnedHandle,
}

impl Channel {
    #[inline]
    pub fn send<M: MessageBody>(&self, message: M) -> Result<(), FtlError> {
        unsafe {
            sys_channel_send(
                self.handle.0,
                message.msginfo(),
                &raw const message as *const u8,
            )?;
        }

        core::mem::forget(message);
        Ok(())
    }
}

pub fn send_ping(handle_id: i32) {
    let ch = Channel {
        handle: OwnedHandle(HandleId(handle_id)),
    };

    ch.send(Ping {
        value: 42,
    });
}

pub fn send_tcp_write(handle_id: i32) {
    let ch = Channel {
        handle: OwnedHandle(HandleId(handle_id)),
    };

    ch.send(TcpWrite {
        data: "Hel!".as_bytes().try_into().unwrap(),
    });
}

pub fn main() {}
