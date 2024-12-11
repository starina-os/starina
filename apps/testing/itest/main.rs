#![no_std]
#![no_main]
#![allow(unused)]

use core::convert::TryInto;

use starina::arch::syscall;
use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::handle::OwnedHandle;
use starina::message::MessageBuffer;
use starina::println;
use starina::syscall::SyscallNumber;

// pub type VsyscallEntry = extern "C" fn(isize, isize, isize, isize, isize, isize) -> isize;
// static mut VSYSCALL_ENTRY: *const VsyscallEntry = core::ptr::null();

#[inline(always)]
fn fast_memcpy(src: *const u8, dst: *mut u8, len: usize) {
    use core::arch::asm;

    unsafe {
        core::ptr::copy_nonoverlapping(src, dst, len);
    }
    // unsafe {
    //     asm!(
    //         "cld;rep movsb",
    //         inout("rcx") (len as isize) => _,
    //         inout("rsi") (src as isize) => _,
    //         inout("rdi") (dst as isize) => _,
    //         options(nostack),
    //     );
    // }
}

#[repr(C)]
#[derive(Debug)]
pub struct Bytes<const CAP: usize> {
    data: [u8; CAP],
}

// impl<const CAP: usize> Bytes<CAP> {
//     #[inline(always)]
//     fn new(data: &[u8]) -> Bytes<CAP> {
//         let copy_len = data.len();
//         debug_assert!(copy_len < CAP);
//         let mut uninit = unsafe { core::mem::MaybeUninit::<Bytes<CAP>>::uninit().assume_init() };
//         uninit.len = copy_len as u16;
//         unsafe {
//             fast_memcpy(data.as_ptr(), uninit.data.as_mut_ptr() as *mut u8, copy_len);
//         }

//         uninit
//     }
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooManyBytesError;

// impl<const CAP: usize> TryInto<Bytes<CAP>> for &[u8] {
//     type Error = TooManyBytesError;

//     fn try_into(self) -> Result<Bytes<CAP>, Self::Error> {
//         if self.len() <= CAP {
//             Ok(Bytes::new(self))
//         } else {
//             Err(TooManyBytesError)
//         }
//     }
// }

pub trait MessageBody {
    fn serialize(self, msgbuffer: &mut MessageBuffer) -> Result<MessageInfo, TooManyBytesError>;
}

#[repr(C)]
#[derive(Debug)]
pub struct Ping {
    value: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct PingRaw {
    value: u32,
}

impl MessageBody for Ping {
    fn serialize(
        mut self,
        msgbuffer: &mut MessageBuffer,
    ) -> Result<MessageInfo, TooManyBytesError> {
        let info = MessageInfo::new(0, core::mem::size_of::<PingRaw>() as u16, 0);

        unsafe {
            core::ptr::write(
                msgbuffer.data.as_mut_ptr() as *mut PingRaw,
                PingRaw { value: self.value },
            )
        }

        core::mem::forget(self);
        Ok(info)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct TcpWriteRaw {
    pub data: Bytes<512>,
}

#[derive(Debug)]
pub struct TcpWrite<'a> {
    pub data: &'a [u8],
}

impl<'a> MessageBody for TcpWrite<'a> {
    fn serialize(
        mut self,
        msgbuffer: &mut MessageBuffer,
    ) -> Result<MessageInfo, TooManyBytesError> {
        let len = self.data.len();
        if (len > 512) {
            return Err(TooManyBytesError);
        }

        unsafe {
            let ptr = msgbuffer.data.as_mut_ptr() as *mut TcpWriteRaw;
            // core::ptr::write(&raw mut (*ptr).data.len, len as u16);
            let data_ptr = (*ptr).data.data.as_mut_slice().as_mut_ptr();
            fast_memcpy(self.data.as_ptr(), data_ptr, len);
            // core::ptr::copy_nonoverlapping(self.data.as_ptr(), data_ptr, len);
        }

        let info = MessageInfo::new(0, 2 as u16 + len as u16, 0);
        core::mem::forget(self);
        Ok(info)
    }
}

// extern "C" {
//     fn sys_channel_send(handle: HandleId, info: MessageInfo, m: *const u8) -> Result<(), ErrorCode>;
// }
fn sys_channel_send(handle: HandleId, info: MessageInfo, m: *const u8) -> Result<(), ErrorCode> {
    syscall(
        SyscallNumber::ChannelSend,
        handle.as_i32() as usize,
        info.0 as usize,
        m as usize,
        0,
        0,
    )?;

    Ok(())
}

fn sys_channel_recv(handle: HandleId, m: *mut u8) -> Result<MessageInfo, ErrorCode> {
    let ret = syscall(
        SyscallNumber::ChannelRecv,
        handle.as_i32() as usize,
        m as usize,
        0,
        0,
        0,
    )?;

    Ok(MessageInfo(ret as u32))
}
#[derive(Debug)]
pub enum SendError {
    Serialize(TooManyBytesError),
    Syscall(ErrorCode),
}

pub struct Channel {
    handle: OwnedHandle,
}

impl Channel {
    #[inline]
    pub fn send<M: MessageBody>(
        &self,
        msgbuffer: &mut MessageBuffer,
        message: M,
    ) -> Result<(), SendError> {
        let msginfo = message.serialize(msgbuffer).map_err(SendError::Serialize)?;
        unsafe {
            sys_channel_send(self.handle.id(), msginfo, msgbuffer.data.as_ptr())
                .map_err(SendError::Syscall)?;
        }

        Ok(())
    }

    #[inline]
    pub fn recv(&self, msgbuffer: &mut MessageBuffer) -> Result<MessageInfo, ErrorCode> {
        unsafe { sys_channel_recv(self.handle.id(), msgbuffer.data.as_mut_ptr()) }
    }
}

#[inline(never)]
pub fn send_ping(ch: &mut Channel, msgbuffer: &mut MessageBuffer) -> Result<(), SendError> {
    ch.send(msgbuffer, Ping { value: 42 })?;

    Ok(())
}

pub fn send_tcp_write(
    msgbuffer: &mut MessageBuffer,
    bytes: &[u8],
    handle_id: i32,
) -> Result<(), SendError> {
    let ch = Channel {
        handle: OwnedHandle::from_raw(HandleId::from_raw(handle_id)),
    };

    let mut msgbuffer = MessageBuffer::new();
    ch.send(&mut msgbuffer, TcpWrite { data: bytes })?;

    Ok(())
}

#[derive(Debug)]
pub enum Message<'a> {
    TcpWrite(TcpWrite<'a>),
    Ping(Ping),
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct MessageInfo(u32);

impl MessageInfo {
    const fn new(id: u32, len: u16, num_handles: u8) -> MessageInfo {
        debug_assert!(id < 1 << 16);
        debug_assert!(len < 1 << 14);
        debug_assert!(num_handles < 1 << 2);
        MessageInfo(id << 16 | (num_handles as u32) << 14 | (len as u32))
    }

    fn all(self) -> u32 {
        self.0
    }

    fn id_and_num_handles_bits(self) -> u32 {
        (self.0 & 0xffff_c000)
    }

    fn len(self) -> u16 {
        (self.0 & 0x3fff) as u16
    }
}

#[no_mangle]
#[inline(never)]
pub fn mainloop2<'a>(
    ch: Channel,
    msgbuffer: &'a mut MessageBuffer,
) -> Result<Message<'a>, ErrorCode> {
    let info = ch.recv(msgbuffer)?;
    let m = match info.id_and_num_handles_bits() {
        0x7890000 => {
            let raw = msgbuffer.data.as_ptr() as *const PingRaw;
            Message::Ping(Ping {
                value: unsafe { (*raw).value },
            })
        }
        0x7900000 if info.len() != 32 => {
            let raw = msgbuffer.data.as_ptr() as *const PingRaw;
            Message::Ping(Ping {
                value: unsafe { (*raw).value },
            })
        }
        _ => {
            // if info.id_and_num_handles_bits() == 0x7890000 {
            //     if info.len() < 512 {
            //         return Err(ErrorCode::Foo);
            //     }

            //     let raw = msgbuffer.data.as_ptr() as *const TcpWriteRaw;
            //     Message::TcpWrite(TcpWrite {
            //         data: unsafe {
            //             core::slice::from_raw_parts(
            //                 (*raw).data.data.as_ptr(),
            //                 info.len() as usize - 2,
            //             )
            //         },
            //     })
            // } else {
                // }
                return Err(ErrorCode::Foo);
        }
    };

    Ok(m)
}

#[no_mangle]
pub fn main(buf: &[u8]) {
    let mut msgbuffer = MessageBuffer::new();
    // send_tcp_write(&mut msgbuffer, buf, 0x1234).unwrap();
    // let mut ch = Channel {
    //     handle: OwnedHandle::from_raw(HandleId::from_raw(0x1234)),
    // };
    // send_ping(&mut ch, &mut msgbuffer).unwrap();

    let m = mainloop2(
        Channel {
            handle: OwnedHandle::from_raw(HandleId::from_raw(0x1234)),
        },
        &mut msgbuffer,
    )
    .unwrap();
    println!("{:?}", m);
}


// pub trait Handler: Sync {
//     fn handle_ping(&self, ctx: MessageCtx, value: u32) {}
//     fn handle_tcp_write(&self, ctx: MessageCtx, data: &[u8]) {}
// }

// pub struct ServerHandler {}

// pub fn main2() {
//     let handler = ServerHandler {};
// }
