use crate::error::FtlError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(isize)]
pub enum SyscallNumber {
    Print = 1,
}

pub struct VsyscallPage {
    pub entry: fn(isize, isize, isize, isize, isize, isize) -> Result<isize, FtlError>,
}
