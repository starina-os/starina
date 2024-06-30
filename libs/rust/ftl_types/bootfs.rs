use core::mem::size_of;

pub const BOOTFS_MAGIC: [u8; 4] = [0xbf, b'B', b'F', b'S'];

#[repr(C)]
pub struct BootfsHeader {
    pub magic: [u8; 4],
    pub num_entries: u32,
}

#[repr(u8)]
pub enum EntryType {
    File = 1,
}

pub const NAME_LEN_MAX: usize = 64 - size_of::<EntryType>() - 2 * size_of::<u32>();

#[repr(C)]
pub struct BootfsEntry {
    pub size: u32,
    pub offset: u32,
    pub entry_type: EntryType,
    /// Null-terminated.
    pub name: [u8; NAME_LEN_MAX],
}
