mod fuse;

use core::cmp::min;
use core::slice;

use fuse::FUSE_FLUSH;
use fuse::FUSE_GETATTR;
use fuse::FUSE_INIT;
use fuse::FUSE_LOOKUP;
use fuse::FUSE_OPEN;
use fuse::FUSE_READ;
use fuse::FUSE_RELEASE;
use fuse::FuseAttr;
use fuse::FuseEntryOut;
use fuse::FuseGetAttrOut;
use fuse::FuseInHeader;
use fuse::FuseInitIn;
use fuse::FuseInitOut;
use fuse::FuseOpenIn;
use fuse::FuseOpenOut;
use fuse::FuseOutHeader;
use fuse::FuseReadIn;
use starina::prelude::*;

use super::device::VirtioDevice;
use super::virtqueue::DescChain;
use super::virtqueue::Virtqueue;
use crate::guest_memory::GuestMemory;

#[repr(C)]
struct VirtioConfig {
    tag: [u8; 36],
    num_request_queues: u32,
    notify_buf_size: u32,
}

pub struct VirtioFs {}

impl VirtioFs {
    pub fn new() -> Self {
        Self {}
    }
}

impl VirtioDevice for VirtioFs {
    fn num_queues(&self) -> u32 {
        3
    }

    fn device_features(&self) -> u64 {
        0
    }

    fn device_id(&self) -> u32 {
        26
    }

    fn vendor_id(&self) -> u32 {
        0
    }

    fn process(&self, memory: &mut GuestMemory, vq: &mut Virtqueue, mut chain: DescChain) {
        info!("virtio-fs: process: chain={:?}", chain);

        let in_header_desc = chain.next_desc(vq, memory).unwrap();
        let in_header = memory
            .read::<FuseInHeader>(in_header_desc.gpaddr())
            .unwrap();

        if in_header.opcode == FUSE_FLUSH || in_header.opcode == FUSE_RELEASE {
            let datain_desc = chain.next_desc(vq, memory).unwrap();
            let out_header_desc = chain.next_desc(vq, memory).unwrap();
            info!("fuse flush");
            memory
                .write(
                    out_header_desc.gpaddr(),
                    FuseOutHeader {
                        len: 0,
                        error: 0,
                        unique: in_header.unique,
                    },
                )
                .unwrap();

            vq.push_used(memory, chain, size_of::<FuseOutHeader>() as u32);

            return;
        }

        let datain_desc = chain.next_desc(vq, memory).unwrap();
        let out_header_desc = chain.next_desc(vq, memory).unwrap();
        let dataout_desc = chain.next_desc(vq, memory).unwrap();
        assert!(in_header_desc.is_read_only());
        assert!(datain_desc.is_read_only());
        assert!(out_header_desc.is_write_only());
        assert!(dataout_desc.is_write_only());

        const HELLO_TEXT: &[u8] = b"Hello from FUSE!";
        const ROOT_DIR_ATTR: FuseAttr = FuseAttr {
            ino: 1,
            size: 0,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            atimensec: 0,
            mtimensec: 0,
            ctimensec: 0,
            mode: 0o755 | 0o40000,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 0,
            padding: 0,
        };
        const HELLO_TXT_ATTR: FuseAttr = FuseAttr {
            ino: 2,
            size: HELLO_TEXT.len() as u64,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            atimensec: 0,
            mtimensec: 0,
            ctimensec: 0,
            mode: 0o100644, // regular file mode
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 0,
            padding: 0,
        };

        let dataout_len = match in_header.opcode {
            FUSE_INIT => {
                info!("fuse init");
                // struct virtio_fs_req {
                //     // Device-readable part
                //     struct fuse_in_header in;
                //     u8 datain[];
                //
                //     // Device-writable part
                //     struct fuse_out_header out;
                //     u8 dataout[];
                // };
                let init_in = memory.read::<FuseInitIn>(datain_desc.gpaddr()).unwrap();
                assert_eq!(init_in.major, 7, "unsupported FUSE version");

                let init_out = FuseInitOut {
                    major: init_in.major,
                    minor: init_in.minor,
                    max_readahead: 0,
                    flags: 0,
                    max_background: 0,
                    congestion_threshold: 0,
                    max_write: 0,
                    time_gran: 0,
                    unused: [0; 9],
                };
                memory.write(dataout_desc.gpaddr(), init_out).unwrap();
                memory
                    .write(
                        out_header_desc.gpaddr(),
                        FuseOutHeader {
                            len: 0,
                            error: 0,
                            unique: in_header.unique,
                        },
                    )
                    .unwrap();
                size_of::<FuseInitOut>()
            }
            FUSE_GETATTR => {
                let getattr_out = if in_header.nodeid == 1 {
                    FuseGetAttrOut {
                        attr: ROOT_DIR_ATTR,
                        attr_valid: 0,
                        attr_valid_nsec: 0,
                        dummy: 0,
                    }
                } else if in_header.nodeid == 2 {
                    FuseGetAttrOut {
                        attr: HELLO_TXT_ATTR,
                        attr_valid: 0,
                        attr_valid_nsec: 0,
                        dummy: 0,
                    }
                } else {
                    panic!("fuse getattr: unknown nodeid: {:x}", in_header.nodeid);
                };

                memory.write(dataout_desc.gpaddr(), getattr_out).unwrap();
                memory
                    .write(
                        out_header_desc.gpaddr(),
                        FuseOutHeader {
                            len: 0,
                            error: 0,
                            unique: in_header.unique,
                        },
                    )
                    .unwrap();
                size_of::<FuseGetAttrOut>()
            }
            FUSE_LOOKUP => {
                let lookup_out = FuseEntryOut {
                    nodeid: 2,
                    generation: 0,
                    entry_valid: 0,
                    attr_valid: 0,
                    entry_valid_nsec: 0,
                    attr_valid_nsec: 0,
                    attr: HELLO_TXT_ATTR,
                };

                memory.write(dataout_desc.gpaddr(), lookup_out).unwrap();
                memory
                    .write(
                        out_header_desc.gpaddr(),
                        FuseOutHeader {
                            len: 0,
                            error: 0,
                            unique: in_header.unique,
                        },
                    )
                    .unwrap();
                size_of::<FuseEntryOut>()
            }
            FUSE_OPEN => {
                let open_in = memory.read::<FuseOpenIn>(datain_desc.gpaddr()).unwrap();
                let open_out = FuseOpenOut {
                    fh: 1,
                    open_flags: open_in.flags,
                    padding: 0,
                };

                memory.write(dataout_desc.gpaddr(), open_out).unwrap();
                memory
                    .write(
                        out_header_desc.gpaddr(),
                        FuseOutHeader {
                            len: 0,
                            error: 0,
                            unique: in_header.unique,
                        },
                    )
                    .unwrap();
                size_of::<FuseOpenOut>()
            }
            FUSE_READ => {
                let read_in = memory.read::<FuseReadIn>(datain_desc.gpaddr()).unwrap();
                let offset = read_in.offset as usize;
                let written_len = min(
                    read_in.size as usize,
                    HELLO_TEXT.len().saturating_sub(offset),
                );
                info!(
                    ">>>>>>>>>>> fuse read: offset={}, written_len={}",
                    offset, written_len
                );
                if written_len > 0 {
                    memory
                        .write_bytes(
                            dataout_desc.gpaddr(),
                            &HELLO_TEXT[offset..offset + written_len as usize],
                        )
                        .unwrap();
                }

                memory
                    .write(
                        out_header_desc.gpaddr(),
                        FuseOutHeader {
                            len: written_len as u32,
                            error: 0,
                            unique: in_header.unique,
                        },
                    )
                    .unwrap();
                written_len
            }
            FUSE_FLUSH => {
                info!("fuse flush");
                memory
                    .write(
                        out_header_desc.gpaddr(),
                        FuseOutHeader {
                            len: 0,
                            error: 0,
                            unique: in_header.unique,
                        },
                    )
                    .unwrap();
                0
            }
            _ => {
                panic!("fuse unknown opcode: {:x}", in_header.opcode);
            }
        };

        let written_len = (size_of::<FuseOutHeader>() + dataout_len)
            .try_into()
            .unwrap();
        vq.push_used(memory, chain, written_len);
    }

    fn config_read(&self, offset: u64, buf: &mut [u8]) {
        let config = VirtioConfig {
            tag: *b"virtfs\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            num_request_queues: 1_u32.to_le(),
            notify_buf_size: 0_u32.to_le(),
        };
        let config_size = size_of::<VirtioConfig>();

        let config_bytes: &[u8] =
            unsafe { slice::from_raw_parts(&config as *const _ as *const u8, config_size) };

        let offset = offset as usize;
        if offset >= config_size {
            debug_warn!("virtio-fs: config read: offset={:x} out of range", offset);
            return;
        }

        let copy_len = min(buf.len(), config_size.saturating_sub(offset));
        buf[..copy_len].copy_from_slice(&config_bytes[offset..offset + copy_len]);
        trace!(
            "virtio-fs: config read: offset={:x}, buf={:x?}, copy_len={}",
            offset, buf, copy_len
        );
    }
}
