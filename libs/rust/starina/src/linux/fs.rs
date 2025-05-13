use core::cmp::min;

use starina::collections::HashMap;
use starina::prelude::*;
use starina::sync::Arc;
use starina::sync::Mutex;

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

#[allow(unused)]
trait Entry {
    fn attr(&self) -> Result<FuseAttr, FuseError>;
    fn lookup(&self, filename: &[u8]) -> Result<Dirent, FuseError>;
    fn read(&self, offset: u64, size: u32, completer: ReadCompleter) -> ReadResult;
    fn write(&self, offset: u64, size: u32, data: &[u8]) -> Result<u32, FuseError>;

    fn open(&self, flags: u32) -> Result<(), FuseError> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct Dirent {
    inode: INode,
    entry: Arc<dyn Entry>,
}

struct RootDirectory {
    files: HashMap<&'static [u8], Dirent>,
}

impl RootDirectory {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn insert_entry(&mut self, inode: INode, name: &'static str, entry: Arc<dyn Entry>) {
        self.files.insert(name.as_bytes(), Dirent { inode, entry });
    }
}

impl Entry for RootDirectory {
    fn attr(&self) -> Result<FuseAttr, FuseError> {
        Ok(FuseAttr {
            size: 0,
            mode: 0o40755, // directory mode
            ..Default::default()
        })
    }

    fn lookup(&self, filename: &[u8]) -> Result<Dirent, FuseError> {
        let dirent = self.files.get(filename).ok_or(FuseError::TODO)?;
        Ok(dirent.clone())
    }

    fn read(&self, _offset: u64, _size: u32, _completer: ReadCompleter) -> ReadResult {
        todo!()
    }

    fn write(&self, _offset: u64, _size: u32, _data: &[u8]) -> Result<u32, FuseError> {
        todo!()
    }
}

struct StaticFile {
    contents: &'static [u8],
}

impl StaticFile {
    pub const fn new(contents: &'static [u8]) -> Self {
        Self { contents }
    }
}

impl Entry for StaticFile {
    fn attr(&self) -> Result<FuseAttr, FuseError> {
        Ok(FuseAttr {
            size: self.contents.len() as u64,
            mode: 0o100644, // regular file mode
            ..Default::default()
        })
    }

    fn lookup(&self, _filename: &[u8]) -> Result<Dirent, FuseError> {
        todo!()
    }

    fn read(&self, offset: u64, size: u32, completer: ReadCompleter) -> ReadResult {
        let file_size = self.contents.len() as usize;
        let offset = offset as usize;
        let read_len = min(size as usize, file_size.saturating_sub(offset));
        let data = &self.contents[offset..offset + read_len];
        completer.complete(data)
    }

    fn write(&self, _offset: u64, _size: u32, _data: &[u8]) -> Result<u32, FuseError> {
        todo!()
    }
}

struct Mutable {
    inodes: HashMap<INode, Arc<dyn Entry>>,
    next_fh: u64,
}

pub struct DemoFileSystem {
    mutable: Mutex<Mutable>,
}

impl DemoFileSystem {
    pub fn new() -> Self {
        let text_txt_id = INode::new(2);

        let mut root_dir = RootDirectory::new();
        root_dir.insert_entry(
            text_txt_id,
            "test.txt",
            Arc::new(StaticFile::new(
                b"Hello from well-refactored FUSE implementaiton!",
            )),
        );

        let mut inodes = HashMap::new();
        inodes.insert(INode::root_dir(), Arc::new(root_dir) as Arc<dyn Entry>);
        inodes.insert(
            text_txt_id,
            Arc::new(StaticFile::new(
                b"Hello from well-refactored FUSE implementaiton!",
            )),
        );

        Self {
            mutable: Mutex::new(Mutable { inodes, next_fh: 1 }),
        }
    }
}

impl virtio_fs::FileSystem for DemoFileSystem {
    fn lookup(&self, dir_inode: INode, filename: &[u8]) -> Result<FuseEntryOut, FuseError> {
        let dirent = self
            .mutable
            .lock()
            .inodes
            .get(&dir_inode)
            .ok_or(FuseError::TODO)?
            .lookup(filename)?;

        let attr = dirent.entry.attr()?;
        Ok(FuseEntryOut {
            nodeid: dirent.inode.0,
            generation: 0,
            entry_valid: 0,
            attr_valid: 0,
            entry_valid_nsec: 0,
            attr_valid_nsec: 0,
            attr,
        })
    }

    fn open(&self, inode: INode, open_in: FuseOpenIn) -> Result<FuseOpenOut, FuseError> {
        let mut mutable = self.mutable.lock();
        mutable
            .inodes
            .get_mut(&inode)
            .ok_or(FuseError::TODO)?
            .open(open_in.flags)?;

        let fh = mutable.next_fh;
        mutable.next_fh += 1;

        Ok(FuseOpenOut {
            fh,
            open_flags: 0,
            padding: 0,
        })
    }

    fn getattr(
        &self,
        inode: INode,
        _getattr_in: FuseGetAttrIn,
    ) -> Result<FuseGetAttrOut, FuseError> {
        let mut attr = self
            .mutable
            .lock()
            .inodes
            .get(&inode)
            .ok_or(FuseError::TODO)?
            .attr()?;

        attr.ino = inode.0;

        Ok(FuseGetAttrOut {
            attr,
            attr_valid: 0,
            attr_valid_nsec: 0,
            dummy: 0,
        })
    }
    fn flush(&self, inode: INode, _flush_in: FuseFlushIn) -> Result<(), FuseError> {
        trace!("flush: inode={:?}", inode);
        Ok(())
    }

    fn release(&self, inode: INode, _release_in: FuseReleaseIn) -> Result<(), FuseError> {
        trace!("release: inode={:?}", inode);
        Ok(())
    }

    fn read(&self, inode: INode, read_in: FuseReadIn, completer: ReadCompleter) -> ReadResult {
        let mutable = self.mutable.lock();
        let Some(entry) = mutable.inodes.get(&inode) else {
            return completer.error(FuseError::TODO);
        };

        entry.read(read_in.offset, read_in.size, completer)
    }

    fn write(
        &self,
        inode: INode,
        write_in: FuseWriteIn,
        data: &[u8],
    ) -> Result<FuseWriteOut, FuseError> {
        let size = self
            .mutable
            .lock()
            .inodes
            .get_mut(&inode)
            .ok_or(FuseError::TODO)?
            .write(write_in.offset, write_in.size, data)?;

        Ok(FuseWriteOut { size, padding: 0 })
    }
}
