use starina::prelude::*;

use super::device::ReadReply;
use super::fuse::FuseEntryOut;
use super::fuse::FuseError;
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

pub struct ReadCompleter<'a>(pub(super) ReadReply<'a>);
pub struct ReadResult(pub(super) Result<usize, guest_memory::Error>);

impl<'a> ReadCompleter<'a> {
    pub fn error(self, error: FuseError) -> ReadResult {
        ReadResult(self.0.reply_error(error))
    }

    pub fn complete(self, data: &[u8]) -> ReadResult {
        ReadResult(self.0.reply(data))
    }
}

pub struct INode(u64);

impl INode {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

pub trait FileSystem {
    fn lookup(&self, dir_inode: INode, filename: &[u8]) -> Result<FuseEntryOut, FuseError>;
    fn open(&self, inode: INode, open_in: FuseOpenIn) -> Result<FuseOpenOut, FuseError>;
    fn getattr(&self, inode: INode, getattr_in: FuseGetAttrIn)
    -> Result<FuseGetAttrOut, FuseError>;
    fn flush(&self, inode: INode, flush_in: FuseFlushIn) -> Result<(), FuseError>;
    fn release(&self, inode: INode, release_in: FuseReleaseIn) -> Result<(), FuseError>;
    fn read(&self, inode: INode, read_in: FuseReadIn, completer: ReadCompleter) -> ReadResult;
    fn write(
        &self,
        inode: INode,
        write_in: FuseWriteIn,
        data: &[u8],
    ) -> Result<FuseWriteOut, FuseError>;
}
