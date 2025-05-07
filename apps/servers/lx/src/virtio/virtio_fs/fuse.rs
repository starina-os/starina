/// `struct fuse_in_header`.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FuseInHeader {
    pub len: u32,
    pub opcode: u32,
    pub unique: u64,
    pub nodeid: u64,
    pub uid: u32,
    pub gid: u32,
    pub pid: u32,
    pub total_extlen: u16,
    pub padding: u16,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FuseOutHeader {
    pub len: u32,
    pub error: i32,
    pub unique: u64,
}

pub const FUSE_LOOKUP: u32 = 1;
pub const FUSE_FORGET: u32 = 2;
pub const FUSE_GETATTR: u32 = 3;
pub const FUSE_SETATTR: u32 = 4;
pub const FUSE_READLINK: u32 = 5;
pub const FUSE_SYMLINK: u32 = 6;
pub const FUSE_MKNOD: u32 = 8;
pub const FUSE_MKDIR: u32 = 9;
pub const FUSE_UNLINK: u32 = 10;
pub const FUSE_RMDIR: u32 = 11;
pub const FUSE_RENAME: u32 = 12;
pub const FUSE_LINK: u32 = 13;
pub const FUSE_OPEN: u32 = 14;
pub const FUSE_READ: u32 = 15;
pub const FUSE_WRITE: u32 = 16;
pub const FUSE_STATFS: u32 = 17;
pub const FUSE_RELEASE: u32 = 18;
pub const FUSE_FSYNC: u32 = 20;
pub const FUSE_SETXATTR: u32 = 21;
pub const FUSE_GETXATTR: u32 = 22;
pub const FUSE_LISTXATTR: u32 = 23;
pub const FUSE_REMOVEXATTR: u32 = 24;
pub const FUSE_FLUSH: u32 = 25;
pub const FUSE_INIT: u32 = 26;
pub const FUSE_OPENDIR: u32 = 27;
pub const FUSE_READDIR: u32 = 28;
pub const FUSE_RELEASEDIR: u32 = 29;
pub const FUSE_FSYNCDIR: u32 = 30;
pub const FUSE_GETLK: u32 = 31;
pub const FUSE_SETLK: u32 = 32;
pub const FUSE_SETLKW: u32 = 33;
pub const FUSE_ACCESS: u32 = 34;
pub const FUSE_CREATE: u32 = 35;
pub const FUSE_INTERRUPT: u32 = 36;
pub const FUSE_BMAP: u32 = 37;
pub const FUSE_DESTROY: u32 = 38;
pub const FUSE_IOCTL: u32 = 39;
pub const FUSE_POLL: u32 = 40;
pub const FUSE_NOTIFY_REPLY: u32 = 41;
pub const FUSE_BATCH_FORGET: u32 = 42;
pub const FUSE_FALLOCATE: u32 = 43;
pub const FUSE_READDIRPLUS: u32 = 44;
pub const FUSE_RENAME2: u32 = 45;
pub const FUSE_LSEEK: u32 = 46;
pub const FUSE_COPY_FILE_RANGE: u32 = 47;
pub const FUSE_SETUPMAPPING: u32 = 48;
pub const FUSE_REMOVEMAPPING: u32 = 49;
pub const FUSE_SYNCFS: u32 = 50;
pub const FUSE_TMPFILE: u32 = 51;
pub const FUSE_STATX: u32 = 52;
