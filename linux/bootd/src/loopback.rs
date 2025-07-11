use std::fs::File;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

use nix::errno::Errno;
use nix::ioctl_write_ptr_bad;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open backing file")]
    BackingFileNotFound,
    #[error("cannot open loop device")]
    LoopDeviceNotFound,
    #[error("failed to configure loop device: {0}")]
    ConfigureFailed(Errno),
}

ioctl_write_ptr_bad!(loop_configure, 0x4c0a, LoopConfig);

#[repr(C)]
struct LoopConfig {
    fd: u32,
    block_size: u32,
    info: LoopInfo64,
}

#[repr(C)]
struct LoopInfo64 {
    device: u64,
    inode: u64,
    rdevice: u64,
    offset: u64,
    sizelimit: u64,
    number: u32,
    encrypt_type: u32,
    encrypt_key_size: u32,
    flags: u32,
    file_name: [u8; 64],
    crypt_name: [u8; 64],
    encrypt_key: [u8; 32],
    init: [u64; 2],
}

pub struct LoopDevice {
    device_path: PathBuf,
}

impl LoopDevice {
    pub fn new(index: u32) -> Self {
        let device_path = PathBuf::from(format!("/dev/loop{}", index));
        LoopDevice { device_path }
    }

    pub fn attach(&mut self, backing_file_path: &str) -> Result<(), Error> {
        let backing_file = OpenOptions::new()
            .read(true)
            .open(backing_file_path)
            .map_err(|_| Error::BackingFileNotFound)?;

        let device_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.device_path)
            .map_err(|_| Error::LoopDeviceNotFound)?;

        let mut info = LoopInfo64 {
            device: 0,
            inode: 0,
            rdevice: 0,
            offset: 0,
            sizelimit: 0,
            number: 0,
            encrypt_type: 0,
            encrypt_key_size: 0,
            flags: 0,
            file_name: [0; 64],
            crypt_name: [0; 64],
            encrypt_key: [0; 32],
            init: [0; 2],
        };

        let backing_file_name = backing_file_path.as_bytes();
        let copy_len = std::cmp::min(backing_file_name.len(), 63);
        info.file_name[..copy_len].copy_from_slice(&backing_file_name[..copy_len]);

        let config = LoopConfig {
            fd: backing_file.as_raw_fd() as u32,
            block_size: 0,
            info,
        };

        unsafe {
            loop_configure(device_file.as_raw_fd(), &config).map_err(Error::ConfigureFailed)?;
        }

        Ok(())
    }

    pub fn device_path(&self) -> &PathBuf {
        &self.device_path
    }
}
