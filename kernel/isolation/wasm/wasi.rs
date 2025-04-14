use alloc::vec::Vec;
use core::mem;
use core::mem::MaybeUninit;
use core::slice;

use wasmi::Caller;
use wasmi::Linker;

use super::HostState;

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct UserPtr(i32);

#[repr(C)]
struct IoVec {
    buf: UserPtr,
    len: u32,
}

pub fn link_wasi(linker: &mut Linker<HostState>) -> Result<(), wasmi::Error> {
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_close",
        |_caller: Caller<'_, HostState>, fd: i32| -> i32 {
            trace!("[wasi] fd_close: fd={}", fd);
            0
        },
    )?;
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_fdstat_get",
        |_caller: Caller<'_, HostState>, fd: i32, buf_ptr: i32| -> i32 {
            trace!("[wasi] fd_fdstat_get: fd={}", fd);
            0
        },
    )?;
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_seek",
        |_caller: Caller<'_, HostState>,
         fd: i32,
         _offset: i64,
         whence: i32,
         newoffset: i32|
         -> i32 {
            trace!("[wasi] fd_seek: fd={}", fd);
            0
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "clock_time_get",
        |_caller: Caller<'_, HostState>, _clock_id: i32, _precision: i64, time_ptr: i32| -> i32 {
            trace!("[wasi] clock_time_get");
            // Return a fixed timestamp (in nanoseconds)
            0
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_fdstat_set_flags",
        |_caller: Caller<'_, HostState>, fd: i32, _flags: i32| -> i32 {
            trace!("[wasi] fd_fdstat_set_flags: fd={}", fd);
            0
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_prestat_get",
        |_caller: Caller<'_, HostState>, fd: i32, _prestat_ptr: i32| -> i32 {
            trace!("[wasi] fd_prestat_get: fd={}", fd);
            // Return EBADF (bad file descriptor)
            8
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_prestat_dir_name",
        |_caller: Caller<'_, HostState>, fd: i32, _path_ptr: i32, _path_len: i32| -> i32 {
            trace!("[wasi] fd_prestat_dir_name: fd={}", fd);
            // Return EBADF (bad file descriptor)
            8
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_read",
        |_caller: Caller<'_, HostState>,
         fd: i32,
         _iovs_ptr: i32,
         _iovs_len: i32,
         _nread_ptr: i32|
         -> i32 {
            trace!("[wasi] fd_read: fd={}", fd);
            // Return 0 bytes read
            0
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_open",
        |_caller: Caller<'_, HostState>,
         fd: i32,
         _dirflags: i32,
         _path_ptr: i32,
         _path_len: i32,
         _oflags: i32,
         _fs_rights_base: i64,
         _fs_rights_inheriting: i64,
         _fdflags: i32,
         _fd_ptr: i32|
         -> i32 {
            trace!("[wasi] path_open: fd={}", fd);
            // Return ENOENT (no such file or directory)
            44
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "poll_oneoff",
        |_caller: Caller<'_, HostState>,
         _in_ptr: i32,
         _out_ptr: i32,
         _nsubscriptions: i32,
         _nevents_ptr: i32|
         -> i32 {
            trace!("[wasi] poll_oneoff");
            // Return 0 events
            0
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_write",
        |mut caller: Caller<'_, HostState>,
         fd: i32,
         iovs_ptr: i32,
         iovs_len: i32,
         written_ptr: i32|
         -> i32 {
            trace!("[wasi] fd_write: fd={}, iov={}", fd, iovs_ptr);
            assert!(fd == 1 || fd == 2);

            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let mut written = 0;
            for i in 0..iovs_len {
                let mut iov: MaybeUninit<IoVec> = MaybeUninit::uninit();
                debug_assert_eq!(mem::size_of::<IoVec>(), mem::size_of_val(&iov));
                let iov_bytes = unsafe {
                    slice::from_raw_parts_mut(iov.as_mut_ptr() as *mut u8, mem::size_of::<IoVec>())
                };

                // Calculate the correct offset for each IoVec in the array
                trace!(
                    "[wasi][iovec] iovs_ptr={:x}, i={}/{}",
                    iovs_ptr, i, iovs_len
                );
                let iov_offset = iovs_ptr + i * (mem::size_of::<IoVec>() as i32);
                memory
                    .read(&caller, iov_offset.try_into().unwrap(), iov_bytes)
                    .unwrap();

                let iov = unsafe { iov.assume_init() };
                let mut buf = Vec::with_capacity(iov.len as usize);
                buf.resize(iov.len as usize, 0);

                trace!("[wasi][iovec] buf={:x}, len={}", iov.buf.0, iov.len);
                memory
                    .read(&caller, iov.buf.0.try_into().unwrap(), &mut buf)
                    .unwrap();

                info!(
                    "[wasi][stdio] \x1b[1;32m{}\x1b[0m",
                    ::core::str::from_utf8(&buf).unwrap()
                );

                let iov_len_i32: i32 = iov.len.try_into().unwrap();
                written += iov_len_i32;
            }

            memory
                .write(
                    &mut caller,
                    written_ptr.try_into().unwrap(),
                    &written.to_le_bytes(),
                )
                .unwrap();

            written
        },
    )?;
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "proc_exit",
        |caller: Caller<'_, HostState>, exit_code: i32| {
            trace!("[wasi] proc_exit: {}", exit_code);
        },
    )?;
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "args_get",
        |_caller: Caller<'_, HostState>, _argv: i32, _argv_buf: i32| -> i32 {
            trace!("[wasi] args_get");
            0
        },
    )?;
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "args_sizes_get",
        |_caller: Caller<'_, HostState>, _argc_ptr: i32, _argv_buf_size_ptr: i32| -> u32 {
            trace!("[wasi] args_sizes_get");
            0
        },
    )?;
    Ok(())
}
