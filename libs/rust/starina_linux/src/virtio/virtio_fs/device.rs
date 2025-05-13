use core::cmp::min;
use core::slice;

use starina::prelude::*;

use super::fs::FileSystem;
use super::fs::INodeNo;
use super::fs::ReadCompleter;
use super::fuse::FUSE_FLUSH;
use super::fuse::FUSE_GETATTR;
use super::fuse::FUSE_INIT;
use super::fuse::FUSE_LOOKUP;
use super::fuse::FUSE_OPEN;
use super::fuse::FUSE_READ;
use super::fuse::FUSE_RELEASE;
use super::fuse::FUSE_WRITE;
use super::fuse::Errno;
use super::fuse::FuseFlushIn;
use super::fuse::FuseGetAttrIn;
use super::fuse::FuseInHeader;
use super::fuse::FuseInitIn;
use super::fuse::FuseInitOut;
use super::fuse::FuseOpenIn;
use super::fuse::FuseOutHeader;
use super::fuse::FuseReadIn;
use super::fuse::FuseReleaseIn;
use super::fuse::FuseWriteIn;
use crate::guest_memory;
use crate::guest_memory::GuestMemory;
use crate::virtio::device::VirtioDevice;
use crate::virtio::virtqueue::DescChain;
use crate::virtio::virtqueue::DescChainReader;
use crate::virtio::virtqueue::DescChainWriter;
use crate::virtio::virtqueue::Virtqueue;

pub struct Reply<'a> {
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

    pub(super) fn do_reply<T: Copy>(
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

        self.desc_writer.write(FuseOutHeader {
            len: len as u32,
            error: 0,
            unique: self.unique,
        })?;

        if let Some(out) = out {
            self.desc_writer.write(out)?;
        }

        if let Some(bytes) = bytes {
            self.desc_writer.write_bytes(bytes)?;
        }

        Ok(len)
    }

    #[track_caller]
    pub fn reply_error(mut self, error: Errno) -> Result<usize, guest_memory::Error> {
        debug_warn!("reply_error from: {:?}", core::panic::Location::caller());
        let len = size_of::<FuseOutHeader>();
        self.desc_writer.write(FuseOutHeader {
            len: len as u32,
            error: error as i32,
            unique: self.unique,
        })?;

        Ok(len)
    }

    pub fn reply_without_data(self) -> Result<usize, guest_memory::Error> {
        self.do_reply(None as Option<()>, None)
    }

    pub fn reply<T: Copy>(self, out: T) -> Result<usize, guest_memory::Error> {
        self.do_reply(Some(out), None)
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
            warn!("virtio-fs: unsupported major version: {:x}", init_in.major);
            return reply.reply_error(Errno::TODO);
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
        let filename_len = match (in_header.len as usize).checked_sub(size_of::<FuseInHeader>()) {
            Some(len) => len,
            None => return reply.reply_error(Errno::TODO),
        };

        let filename_with_nulls = reader.read_zerocopy(filename_len)?;

        // The filename may be terminated by NULL. Trim them.
        let trailing_nulls = filename_with_nulls
            .iter()
            .rev()
            .take_while(|&b| *b == 0)
            .count();
        let filename = &filename_with_nulls[..filename_with_nulls.len() - trailing_nulls];

        let dir_ino = INodeNo::new(in_header.nodeid);
        match self.fs.lookup(dir_ino, filename) {
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
        let ino = INodeNo::new(in_header.nodeid);
        match self.fs.open(ino, open_in) {
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
        let node_id = INodeNo::new(in_header.nodeid);
        match self.fs.getattr(node_id, getattr_in) {
            Ok(out) => reply.reply(out),
            Err(e) => reply.reply_error(e),
        }
    }

    fn do_flush(
        &self,
        in_header: FuseInHeader,
        mut reader: DescChainReader<'_>,
        reply: Reply<'_>,
    ) -> Result<usize, guest_memory::Error> {
        let flush_in = reader.read::<FuseFlushIn>()?;
        let node_id = INodeNo::new(in_header.nodeid);
        match self.fs.flush(node_id, flush_in) {
            Ok(()) => reply.reply_without_data(),
            Err(e) => reply.reply_error(e),
        }
    }

    fn do_release(
        &self,
        in_header: FuseInHeader,
        mut reader: DescChainReader<'_>,
        reply: Reply<'_>,
    ) -> Result<usize, guest_memory::Error> {
        let release_in = reader.read::<FuseReleaseIn>()?;
        let node_id = INodeNo::new(in_header.nodeid);
        match self.fs.release(node_id, release_in) {
            Ok(()) => reply.reply_without_data(),
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
        let node_id = INodeNo::new(in_header.nodeid);
        let read_reply = ReadCompleter(reply);
        let result = self.fs.read(node_id, read_in, read_reply);
        result.0
    }

    fn do_write(
        &self,
        in_header: FuseInHeader,
        mut reader: DescChainReader<'_>,
        reply: Reply<'_>,
    ) -> Result<usize, guest_memory::Error> {
        let write_in = reader.read::<FuseWriteIn>()?;
        let node_id = INodeNo::new(in_header.nodeid);
        let len = match (write_in.size as usize).checked_sub(size_of::<FuseWriteIn>()) {
            Some(len) => len,
            None => return reply.reply_error(Errno::TODO),
        };

        let buf = reader.read_zerocopy(len)?;
        match self.fs.write(node_id, write_in, buf) {
            Ok(out) => reply.reply(out),
            Err(e) => reply.reply_error(e),
        }
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

    /// Handles a FUSE request from the guest.
    ///
    /// # Endianess
    ///
    /// Virtio spec says "The endianness of the FUSE protocol session is
    /// detectable by inspecting the uint32_t in.opcode field of the FUSE_INIT
    /// request sent by the driver to the device".
    ///
    /// In this implementation, we assume the guest and the host use the same
    /// endianness. We don't have a check for the endianness but the guest
    /// should fail because we will handle their FUSE_INIT as an invalid request.
    fn process(&self, memory: &mut GuestMemory, vq: &mut Virtqueue, chain: DescChain) {
        let (mut reader, writer) = chain.reader_writer(vq, memory).unwrap();

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
            FUSE_FLUSH => self.do_flush(in_header, reader, reply),
            FUSE_RELEASE => self.do_release(in_header, reader, reply),
            FUSE_READ => self.do_read(in_header, reader, reply),
            FUSE_WRITE => self.do_write(in_header, reader, reply),
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
