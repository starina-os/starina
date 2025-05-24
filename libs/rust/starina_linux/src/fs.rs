use starina::collections::HashMap;
use starina::prelude::*;
use starina::sync::Arc;
use starina::sync::Mutex;

use crate::ReadDirCompleter;
use crate::virtio::virtio_fs;
use crate::virtio::virtio_fs::INodeNo;
use crate::virtio::virtio_fs::ReadCompleter;
use crate::virtio::virtio_fs::ReadResult;
use crate::virtio::virtio_fs::fuse::Errno;
use crate::virtio::virtio_fs::fuse::FuseAttr;
use crate::virtio::virtio_fs::fuse::FuseDirent;
use crate::virtio::virtio_fs::fuse::FuseEntryOut;
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
pub trait INode {
    fn attr(&self) -> Result<FuseAttr, Errno>;
    fn lookup(&self, filename: &[u8]) -> Result<Dirent, Errno>;
    fn read(&self, offset: u64, size: u32, completer: ReadCompleter) -> ReadResult;
    fn readdir(&self, offset: u64, size: u32, completer: ReadDirCompleter) -> ReadResult;
    fn write(&self, offset: u64, data: &[u8]) -> Result<u32, Errno>;

    fn open(&self, flags: u32) -> Result<(), Errno> {
        Ok(())
    }
}

pub trait FileLike: INode {
    fn size(&self) -> usize;
    fn read_at(&self, offset: usize, size: usize, completer: ReadCompleter) -> ReadResult;
    fn write_at(&self, offset: usize, data: &[u8]) -> Result<usize, Errno>;
}

impl<T: FileLike> INode for T {
    fn attr(&self) -> Result<FuseAttr, Errno> {
        Ok(FuseAttr {
            size: self.size() as u64,
            mode: 0o100644, // regular file mode
            ..Default::default()
        })
    }

    fn lookup(&self, _filename: &[u8]) -> Result<Dirent, Errno> {
        todo!()
    }

    fn read(&self, offset: u64, size: u32, completer: ReadCompleter) -> ReadResult {
        self.read_at(offset as usize, size as usize, completer)
    }

    fn write(&self, offset: u64, data: &[u8]) -> Result<u32, Errno> {
        let len = self.write_at(offset as usize, data)?;
        Ok(len as u32)
    }

    fn readdir(&self, _offset: u64, _size: u32, completer: ReadDirCompleter) -> ReadResult {
        completer.error(Errno::ENOTDIR)
    }
}

#[derive(Clone)]
pub struct Dirent {
    ino: INodeNo,
    entry: Arc<dyn INode>,
}

struct Directory {
    files: HashMap<Vec<u8>, Dirent>,
}

impl Directory {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn insert_entry<S: Into<Vec<u8>>>(&mut self, ino: INodeNo, name: S, entry: Arc<dyn INode>) {
        self.files.insert(name.into(), Dirent { ino, entry });
    }
}

impl INode for Directory {
    fn attr(&self) -> Result<FuseAttr, Errno> {
        Ok(FuseAttr {
            size: 0,
            mode: 0o40755, // directory mode
            ..Default::default()
        })
    }

    fn lookup(&self, filename: &[u8]) -> Result<Dirent, Errno> {
        let dirent = self.files.get(filename).ok_or(Errno::TODO)?;
        Ok(dirent.clone())
    }

    fn readdir(&self, _offset: u64, _size: u32, completer: ReadDirCompleter) -> ReadResult {
        info!("readdir: offset={}, size={}", _offset, _size);
        let Some((name, dirent)) = self.files.iter().skip(0).next() else {
            return completer.complete_with_eof();
        };

        const DT_REG: u32 = 0x08;

        completer.complete(
            FuseDirent {
                ino: dirent.ino.0,
                off: 0,
                namelen: name.len() as u32,
                file_type: DT_REG,
            },
            name,
        )
    }

    fn read(&self, _offset: u64, _size: u32, _completer: ReadCompleter) -> ReadResult {
        todo!()
    }

    fn write(&self, _offset: u64, _data: &[u8]) -> Result<u32, Errno> {
        todo!()
    }
}

struct Mutable {
    inodes: HashMap<INodeNo, Arc<dyn INode>>,
    next_fh: u64,
}

pub struct FileSystemBuilder {
    next_ino: u64,
    root_dir: Directory,
}

impl FileSystemBuilder {
    pub fn new() -> Self {
        Self {
            next_ino: 2,
            root_dir: Directory::new(),
        }
    }

    pub fn add_root_file(&mut self, name: &str, file: Arc<dyn FileLike>) {
        let ino = INodeNo::new(self.next_ino);
        self.next_ino += 1;
        self.root_dir
            .insert_entry(ino, name, file as Arc<dyn INode>);
    }

    pub fn build(self) -> FileSystem {
        let mut inodes = HashMap::with_capacity(self.root_dir.files.len() + 1);
        for dirent in self.root_dir.files.values() {
            inodes.insert(dirent.ino, dirent.entry.clone());
        }

        inodes.insert(
            INodeNo::root_dir(),
            Arc::new(self.root_dir) as Arc<dyn INode>,
        );

        FileSystem {
            mutable: Mutex::new(Mutable { inodes, next_fh: 1 }),
        }
    }
}
pub struct FileSystem {
    mutable: Mutex<Mutable>,
}

impl virtio_fs::FileSystem for FileSystem {
    fn lookup(&self, dir_ino: INodeNo, filename: &[u8]) -> Result<FuseEntryOut, Errno> {
        let dirent = self
            .mutable
            .lock()
            .inodes
            .get(&dir_ino)
            .ok_or(Errno::TODO)?
            .lookup(filename)?;

        let attr = dirent.entry.attr()?;
        Ok(FuseEntryOut {
            nodeid: dirent.ino.0,
            generation: 0,
            entry_valid: 0,
            attr_valid: 0,
            entry_valid_nsec: 0,
            attr_valid_nsec: 0,
            attr,
        })
    }

    fn open(&self, ino: INodeNo, open_in: FuseOpenIn) -> Result<FuseOpenOut, Errno> {
        let mut mutable = self.mutable.lock();
        mutable
            .inodes
            .get_mut(&ino)
            .ok_or(Errno::TODO)?
            .open(open_in.flags)?;

        let fh = mutable.next_fh;
        mutable.next_fh += 1;

        Ok(FuseOpenOut {
            fh,
            open_flags: 0,
            padding: 0,
        })
    }

    fn getattr(&self, ino: INodeNo, _getattr_in: FuseGetAttrIn) -> Result<FuseGetAttrOut, Errno> {
        let mut attr = self
            .mutable
            .lock()
            .inodes
            .get(&ino)
            .ok_or(Errno::TODO)?
            .attr()?;

        attr.ino = ino.0;

        Ok(FuseGetAttrOut {
            attr,
            attr_valid: 0,
            attr_valid_nsec: 0,
            dummy: 0,
        })
    }
    fn flush(&self, ino: INodeNo, _flush_in: FuseFlushIn) -> Result<(), Errno> {
        trace!("flush: inode={:?}", ino);
        Ok(())
    }

    fn release(&self, ino: INodeNo, _release_in: FuseReleaseIn) -> Result<(), Errno> {
        trace!("release: inode={:?}", ino);
        Ok(())
    }

    fn read(&self, ino: INodeNo, read_in: FuseReadIn, completer: ReadCompleter) -> ReadResult {
        let mutable = self.mutable.lock();
        let Some(entry) = mutable.inodes.get(&ino) else {
            return completer.error(Errno::TODO);
        };

        entry.read(read_in.offset, read_in.size, completer)
    }

    fn readdir(
        &self,
        ino: INodeNo,
        read_in: FuseReadIn,
        completer: ReadDirCompleter,
    ) -> ReadResult {
        let mutable = self.mutable.lock();
        let Some(entry) = mutable.inodes.get(&ino) else {
            return completer.error(Errno::TODO);
        };

        entry.readdir(read_in.offset, read_in.size, completer)
    }

    fn write(
        &self,
        ino: INodeNo,
        write_in: FuseWriteIn,
        data: &[u8],
    ) -> Result<FuseWriteOut, Errno> {
        let size = self
            .mutable
            .lock()
            .inodes
            .get_mut(&ino)
            .ok_or(Errno::TODO)?
            .write(write_in.offset, data)?;

        Ok(FuseWriteOut { size, padding: 0 })
    }
}
