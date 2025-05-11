use core::cmp::min;
use core::mem::MaybeUninit;
use core::slice;

use starina::prelude::*;

use super::fs::FileSystem;
use super::fs::INode;
use super::fs::ReadCompleter;
use super::fs::ReadResult;
use super::fuse::FUSE_FLUSH;
use super::fuse::FUSE_GETATTR;
use super::fuse::FUSE_INIT;
use super::fuse::FUSE_LOOKUP;
use super::fuse::FUSE_OPEN;
use super::fuse::FUSE_READ;
use super::fuse::FUSE_RELEASE;
use super::fuse::FuseAttr;
use super::fuse::FuseEntryOut;
use super::fuse::FuseError;
use super::fuse::FuseGetAttrIn;
use super::fuse::FuseGetAttrOut;
use super::fuse::FuseInHeader;
use super::fuse::FuseInitIn;
use super::fuse::FuseInitOut;
use super::fuse::FuseOpenIn;
use super::fuse::FuseOpenOut;
use super::fuse::FuseOutHeader;
use super::fuse::FuseReadIn;
use crate::guest_memory;
use crate::guest_memory::GuestMemory;
use crate::virtio::device::VirtioDevice;
use crate::virtio::virtqueue::DescChain;
use crate::virtio::virtqueue::DescChainReader;
use crate::virtio::virtqueue::DescChainWriter;
use crate::virtio::virtqueue::Virtqueue;

const FILENAME_LEN_MAX: usize = 256;

struct Reply<'a> {
    unique: u64,
    desc_writer: DescChainWriter<'a>,
}

impl<'a> Reply<'a> {
    pub fn new(desc_writer: DescChainWriter<'a>, unique: u64) -> Self {
        Self {
            desc_writer,
            unique,
        }
    }

    fn do_reply<T: Copy>(
        mut self,
        out: Option<T>,
        bytes: Option<&[u8]>,
    ) -> Result<usize, guest_memory::Error> {
        let mut len = size_of::<FuseOutHeader>();
        if out.is_some() {
            len += size_of::<T>();
        }
        if let Some(bytes) = bytes {
            len += bytes.len();
        }

        let out_header = FuseOutHeader {
            len: len as u32,
            error: 0,
            unique: self.unique,
        };

        self.desc_writer.write(out_header)?;

        if let Some(out) = out {
            self.desc_writer.write(out)?;
        }

        if let Some(bytes) = bytes {
            self.desc_writer.write(bytes)?;
        }

        Ok(len)
    }

    pub fn reply_error(&self, error: FuseError) -> Result<usize, guest_memory::Error> {
        todo!()
    }

    pub fn reply<T: Copy>(self, out: T) -> Result<usize, guest_memory::Error> {
        self.do_reply(Some(out), None)
    }
}

pub struct ReadReply<'a> {
    inner: Reply<'a>,
}

impl<'a> ReadReply<'a> {
    pub fn new(inner: Reply<'a>) -> Self {
        Self { inner }
    }

    pub fn reply_error(&self, error: FuseError) -> Result<usize, guest_memory::Error> {
        self.inner.reply_error(error)
    }

    pub fn reply(self, data: &[u8]) -> Result<usize, guest_memory::Error> {
        self.inner.do_reply(None as Option<()>, Some(data))
    }
}

#[repr(C)]
struct VirtioConfig {
    tag: [u8; 36],
    num_request_queues: u32,
    notify_buf_size: u32,
}

pub struct VirtioFs {
    fs: Box<dyn FileSystem>,
}

impl VirtioFs {
    pub fn new(fs: Box<dyn FileSystem>) -> Self {
        Self { fs }
    }

    fn do_init(
        &self,
        mut reader: DescChainReader<'_>,
        reply: Reply<'_>,
    ) -> Result<usize, guest_memory::Error> {
        let init_in = reader.read::<FuseInitIn>()?;

        if init_in.major != 7 {
            return reply.reply_error(FuseError::TODO);
        }

        reply.reply(FuseInitOut {
            major: init_in.major,
            minor: init_in.minor,
            max_readahead: 0,
            flags: 0,
            max_background: 0,
            congestion_threshold: 0,
            max_write: 0,
            time_gran: 0,
            unused: [0; 9],
        })
    }

    fn do_lookup(
        &self,
        in_header: FuseInHeader,
        mut reader: DescChainReader<'_>,
        reply: Reply<'_>,
    ) -> Result<usize, guest_memory::Error> {
        let filename_len = (in_header.len as usize).saturating_sub(size_of::<FuseInHeader>());
        if filename_len > FILENAME_LEN_MAX {
            return reply.reply_error(FuseError::TODO);
        }

        let mut filename_buf = [0; FILENAME_LEN_MAX]; // TODO: Use MaybeUninit
        reader.read_bytes(filename_buf.as_mut_slice())?;
        let filename = &filename_buf[..filename_len];

        let dir_inode = INode::new(in_header.nodeid);
        match self.fs.lookup(dir_inode, filename) {
            Ok(entry) => reply.reply(entry),
            Err(e) => reply.reply_error(e),
        }
    }

    fn do_open(
        &self,
        in_header: FuseInHeader,
        mut reader: DescChainReader<'_>,
        reply: Reply<'_>,
    ) -> Result<usize, guest_memory::Error> {
        let open_in = reader.read::<FuseOpenIn>()?;
        let node_id = INode::new(in_header.nodeid);
        match self.fs.open(node_id, open_in) {
            Ok(out) => reply.reply(out),
            Err(e) => reply.reply_error(e),
        }
    }

    fn do_getattr(
        &self,
        in_header: FuseInHeader,
        mut reader: DescChainReader<'_>,
        reply: Reply<'_>,
    ) -> Result<usize, guest_memory::Error> {
        let getattr_in = reader.read::<FuseGetAttrIn>()?;
        let node_id = INode::new(in_header.nodeid);
        match self.fs.getattr(node_id, getattr_in) {
            Ok(out) => reply.reply(out),
            Err(e) => reply.reply_error(e),
        }
    }

    fn do_read(
        &self,
        in_header: FuseInHeader,
        mut reader: DescChainReader<'_>,
        reply: Reply<'_>,
    ) -> Result<usize, guest_memory::Error> {
        let read_in = reader.read::<FuseReadIn>()?;
        let node_id = INode::new(in_header.nodeid);
        let read_reply = ReadCompleter(ReadReply::new(reply));
        self.fs.read(node_id, read_in, read_reply).0
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
        let (mut reader, mut writer) = chain.reader_writer(vq, memory).unwrap();

        let in_header = match reader.read::<FuseInHeader>() {
            Ok(in_header) => in_header,
            Err(e) => {
                debug_warn!("failed to read fuse_in header: {:?}", e);
                return;
            }
        };

        let reply = Reply::new(writer, in_header.unique);
        let result = match in_header.opcode {
            FUSE_INIT => self.do_init(reader, reply),
            FUSE_LOOKUP => self.do_lookup(in_header, reader, reply),
            FUSE_OPEN => self.do_open(in_header, reader, reply),
            FUSE_GETATTR => self.do_getattr(in_header, reader, reply),
            FUSE_READ => self.do_read(in_header, reader, reply),
            _ => {
                debug_warn!("virtio-fs: unknown opcode: {:x}", in_header.opcode);
                return;
            }
        };

        match result {
            Ok(written_len) => vq.push_used(memory, chain, written_len as u32),
            Err(e) => {
                debug_warn!("virtio-fs: failed to process request: {:?}", e);
            }
        }
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
    }
}
