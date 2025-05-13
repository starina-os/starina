use super::device::Reply;
use super::fuse::Errno;
use super::fuse::FuseEntryOut;
use super::fuse::FuseFlushIn;
use super::fuse::FuseGetAttrIn;
use super::fuse::FuseGetAttrOut;
use super::fuse::FuseOpenIn;
use super::fuse::FuseOpenOut;
use super::fuse::FuseReadIn;
use super::fuse::FuseReleaseIn;
use super::fuse::FuseWriteIn;
use super::fuse::FuseWriteOut;
use crate::guest_memory;

pub struct ReadCompleter<'a>(pub(super) Reply<'a>);
pub struct ReadResult(pub(super) Result<usize, guest_memory::Error>);

impl<'a> ReadCompleter<'a> {
    pub fn error(self, error: Errno) -> ReadResult {
        let result = self.0.reply_error(error);
        ReadResult(result)
    }

    pub fn complete(self, data: &[u8]) -> ReadResult {
        let result = self.0.do_reply(None as Option<()>, Some(data));
        ReadResult(result)
    }
}

/// The inode number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct INodeNo(pub u64);

impl INodeNo {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub const fn root_dir() -> Self {
        // As defined in `FUSE_ROOT_ID`.
        Self(1)
    }
}

pub trait FileSystem {
    fn lookup(&self, dir_ino: INodeNo, filename: &[u8]) -> Result<FuseEntryOut, Errno>;
    fn open(&self, ino: INodeNo, open_in: FuseOpenIn) -> Result<FuseOpenOut, Errno>;
    fn getattr(&self, ino: INodeNo, getattr_in: FuseGetAttrIn) -> Result<FuseGetAttrOut, Errno>;
    fn flush(&self, ino: INodeNo, flush_in: FuseFlushIn) -> Result<(), Errno>;
    fn release(&self, ino: INodeNo, release_in: FuseReleaseIn) -> Result<(), Errno>;
    fn read(&self, ino: INodeNo, read_in: FuseReadIn, completer: ReadCompleter) -> ReadResult;
    fn write(
        &self,
        ino: INodeNo,
        write_in: FuseWriteIn,
        data: &[u8],
    ) -> Result<FuseWriteOut, Errno>;
}
