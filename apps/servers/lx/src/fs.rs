use core::cmp::min;

use starina::prelude::*;

use crate::virtio::virtio_fs;
use crate::virtio::virtio_fs::INode;
use crate::virtio::virtio_fs::ReadCompleter;
use crate::virtio::virtio_fs::ReadResult;
use crate::virtio::virtio_fs::fuse::FuseAttr;
use crate::virtio::virtio_fs::fuse::FuseEntryOut;
use crate::virtio::virtio_fs::fuse::FuseError;
use crate::virtio::virtio_fs::fuse::FuseFlushIn;
use crate::virtio::virtio_fs::fuse::FuseGetAttrIn;
use crate::virtio::virtio_fs::fuse::FuseGetAttrOut;
use crate::virtio::virtio_fs::fuse::FuseOpenIn;
use crate::virtio::virtio_fs::fuse::FuseOpenOut;
use crate::virtio::virtio_fs::fuse::FuseReadIn;
use crate::virtio::virtio_fs::fuse::FuseReleaseIn;
use crate::virtio::virtio_fs::fuse::FuseWriteIn;
use crate::virtio::virtio_fs::fuse::FuseWriteOut;

pub struct DemoFileSystem {}

impl DemoFileSystem {
    pub fn new() -> Self {
        Self {}
    }
}

const HELLO_TEXT: &[u8] = b"Hello from FUSE2!";
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

impl virtio_fs::FileSystem for DemoFileSystem {
    fn lookup(&self, dir_inode: INode, filename: &[u8]) -> Result<FuseEntryOut, FuseError> {
        let filename = match str::from_utf8(filename) {
            Ok(s) => s,
            Err(_) => {
                debug_warn!("lookup: non-UTF-8 filename: {:x?}", filename);
                return Err(FuseError::TODO);
            }
        };

        trace!(
            "lookup: dir_inode={:?}, filename=\"{}\"",
            dir_inode, filename
        );
        Ok(FuseEntryOut {
            nodeid: 2,
            generation: 0,
            entry_valid: 0,
            attr_valid: 0,
            entry_valid_nsec: 0,
            attr_valid_nsec: 0,
            attr: HELLO_TXT_ATTR,
        })
    }

    fn open(&self, inode: INode, open_in: FuseOpenIn) -> Result<FuseOpenOut, FuseError> {
        assert!(inode == INode::new(2));
        Ok(FuseOpenOut {
            fh: 1,
            open_flags: 0,
            padding: 0,
        })
    }

    fn getattr(
        &self,
        inode: INode,
        getattr_in: FuseGetAttrIn,
    ) -> Result<FuseGetAttrOut, FuseError> {
        let attr = if inode == INode::new(1) {
            FuseAttr {
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
            }
        } else if inode == INode::new(2) {
            HELLO_TXT_ATTR
        } else {
            panic!("Invalid inode: {:?}", inode);
        };

        Ok(FuseGetAttrOut {
            attr,
            attr_valid: 0,
            attr_valid_nsec: 0,
            dummy: 0,
        })
    }

    fn flush(&self, inode: INode, flush_in: FuseFlushIn) -> Result<(), FuseError> {
        trace!("flush: inode={:?}", inode);
        Ok(())
    }

    fn release(&self, inode: INode, release_in: FuseReleaseIn) -> Result<(), FuseError> {
        trace!("release: inode={:?}", inode);
        Ok(())
    }

    fn read(&self, inode: INode, read_in: FuseReadIn, completer: ReadCompleter) -> ReadResult {
        let file_size = HELLO_TEXT.len() as usize;
        let offset = read_in.offset as usize;
        let read_len = min(read_in.size as usize, file_size.saturating_sub(offset));
        let data = &HELLO_TEXT[offset..offset + read_len];

        info!(
            "read: inode={:?}, offset={}, read_len={}, data={:02x?}",
            inode, offset, read_len, data
        );
        completer.complete(data)
    }

    fn write(
        &self,
        inode: INode,
        write_in: FuseWriteIn,
        data: &[u8],
    ) -> Result<FuseWriteOut, FuseError> {
        todo!()
    }
}
