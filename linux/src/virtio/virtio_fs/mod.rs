mod device;
mod fs;
pub mod fuse;

pub use device::VirtioFs;
pub use fs::FileSystem;
pub use fs::INodeNo;
pub use fs::ReadCompleter;
pub use fs::ReadDirCompleter;
pub use fs::ReadResult;
use thiserror::Error;

use crate::guest_memory;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to access guest memory: {0}")]
    GuestMemory(#[from] guest_memory::Error),
}
