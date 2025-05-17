//! FUSE protocol.
//!
//! <https://man7.org/linux/man-pages/man4/fuse.4.html>
#![allow(unused)]

#[derive(Debug, Clone, Copy)]
#[repr(i32)]
#[allow(non_camel_case_types)]
pub enum Errno {
    TODO,
    EACCES = -13,
    ENOTDIR = -20,
    EINVAL = -22,
    EHOSTDOWN = -112,
    EOPNOTSUPP = -95,
}

/// `struct fuse_in_header`.
#[derive(Debug, Clone, Copy, Default)]
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

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseOutHeader {
    pub len: u32,
    pub error: i32,
    pub unique: u64,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseInitIn {
    pub major: u32,
    pub minor: u32,
    /// Since v7.6.
    pub max_readahead: u32,
    /// Since v7.6.
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseInitOut {
    pub major: u32,
    pub minor: u32,
    /// Since v7.6.
    pub max_readahead: u32,
    /// Since v7.6.
    pub flags: u32,
    /// Since v7.13.
    pub max_background: u16,
    /// Since v7.13.
    pub congestion_threshold: u16,
    /// Since v7.5.
    pub max_write: u32,
    /// Since v7.6.
    pub time_gran: u32,
    pub unused: [u32; 9],
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseFlushIn {
    pub fh: u64,
    pub unused: u32,
    pub padding: u32,
    pub lock_owner: u64,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseReleaseIn {
    pub fh: u64,
    pub flags: u32,
    pub release_flags: u32,
    pub lock_owner: u64,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseGetAttrIn {
    pub getattr_flags: u32,
    pub dummy: u32,
    pub fh: u64,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseGetAttrOut {
    pub attr_valid: u64,
    pub attr_valid_nsec: u32,
    pub dummy: u32,
    pub attr: FuseAttr,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseAttr {
    pub ino: u64,
    pub size: u64,
    pub blocks: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub atimensec: u32,
    pub mtimensec: u32,
    pub ctimensec: u32,
    pub mode: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u32,
    pub blksize: u32,
    pub padding: u32,
}

/// `struct fuse_entry_out`.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseEntryOut {
    pub nodeid: u64,
    pub generation: u64,
    pub entry_valid: u64,
    pub attr_valid: u64,
    pub entry_valid_nsec: u32,
    pub attr_valid_nsec: u32,
    pub attr: FuseAttr,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseOpenIn {
    pub flags: u32,
    pub unused: u32,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseOpenOut {
    pub fh: u64,
    pub open_flags: u32,
    pub padding: u32,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseReadIn {
    pub fh: u64,
    pub offset: u64,
    pub size: u32,
    pub read_flags: u32,
    pub lock_owner: u64,
    pub flags: u32,
    pub padding: u32,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseWriteIn {
    pub fh: u64,
    pub offset: u64,
    pub size: u32,
    pub write_flags: u32,
    pub lock_owner: u64,
    pub flags: u32,
    pub padding: u32,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseWriteOut {
    pub size: u32,
    pub padding: u32,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FuseDirent {
    pub ino: u64,
    pub off: u64,
    pub namelen: u32,
    pub file_type: u32,
}

// FUSE operations.
pub const FUSE_LOOKUP: u32 = 1;
pub const FUSE_GETATTR: u32 = 3;
pub const FUSE_OPEN: u32 = 14;
pub const FUSE_READ: u32 = 15;
pub const FUSE_WRITE: u32 = 16;
pub const FUSE_RELEASE: u32 = 18;
pub const FUSE_GETXATTR: u32 = 22;
pub const FUSE_FLUSH: u32 = 25;
pub const FUSE_INIT: u32 = 26;
pub const FUSE_READDIR: u32 = 28;
