use core::cmp::min;

use serde::Deserialize;
use serde::Serialize;
use starina::channel::Channel;
use starina::prelude::*;
use starina::sync::Arc;
use starina::sync::Mutex;
use starina::sync::MutexGuard;

use crate::Errno;
use crate::ReadCompleter;
use crate::ReadResult;
use crate::boot::boot_linux;
use crate::fs::FileLike;
use crate::fs::FileSystemBuilder;
use crate::port_forward::Port;

#[derive(Debug, Serialize, Deserialize)]
struct CommandJson {
    program: String,
    args: Vec<String>,
}

pub struct BufferedStdin(Vec<u8>);

impl BufferedStdin {
    pub fn new<T: Into<Vec<u8>>>(text: T) -> Arc<Self> {
        Arc::new(Self(text.into()))
    }
}

impl FileLike for BufferedStdin {
    fn size(&self) -> usize {
        self.0.len()
    }

    fn read_at(&self, offset: usize, size: usize, completer: ReadCompleter) -> ReadResult {
        if offset > self.0.len() {
            return completer.error(Errno::EINVAL);
        }

        let read_len = min(size, self.0.len() - offset);

        let slice = &self.0[offset..offset + read_len];
        completer.complete(slice)
    }

    fn write_at(&self, _offset: usize, _data: &[u8]) -> Result<usize, Errno> {
        Err(Errno::EACCES)
    }
}

pub struct BufferedStdout(Mutex<Vec<u8>>);

impl BufferedStdout {
    pub fn new() -> Arc<Self> {
        Arc::new(Self(Mutex::new(Vec::new())))
    }

    pub fn buffer(&self) -> MutexGuard<'_, Vec<u8>> {
        self.0.lock()
    }
}

impl FileLike for BufferedStdout {
    fn size(&self) -> usize {
        0
    }

    fn read_at(&self, _offset: usize, _size: usize, _completer: ReadCompleter) -> ReadResult {
        todo!()
    }

    fn write_at(&self, _offset: usize, data: &[u8]) -> Result<usize, Errno> {
        self.0.lock().extend_from_slice(data);
        Ok(data.len())
    }
}

struct RootfsProvider {
    image: &'static [u8],
}

impl RootfsProvider {
    pub fn from_static(image: &'static [u8]) -> Self {
        Self { image }
    }
}

impl FileLike for RootfsProvider {
    fn size(&self) -> usize {
        self.image.len()
    }

    fn read_at(&self, offset: usize, size: usize, completer: ReadCompleter) -> ReadResult {
        if offset > self.image.len() {
            return completer.error(Errno::EINVAL);
        }

        let read_len = min(size, self.image.len() - offset);
        completer.complete(&self.image[offset..offset + read_len])
    }

    fn write_at(&self, _offset: usize, _data: &[u8]) -> Result<usize, Errno> {
        debug_warn!("attempt to write to rootfs");
        Err(Errno::EROFS)
    }
}

#[derive(Debug)]
pub enum Error {}

pub enum ContainerImage {
    Static(&'static [u8]),
}

pub struct Command {
    program: String,
    args: Vec<String>,
    stdin: Option<Arc<dyn FileLike>>,
    stdout: Option<Arc<dyn FileLike>>,
    ports: Vec<Port>,
    image: Option<ContainerImage>,
}

impl Command {
    pub fn new(program: &str) -> Command {
        Command {
            program: program.to_string(),
            args: Vec::new(),
            stdin: None,
            stdout: None,
            ports: Vec::new(),
            image: None,
        }
    }

    pub fn image(&mut self, image: ContainerImage) -> &mut Command {
        self.image = Some(image);
        self
    }

    pub fn arg<S: AsRef<str>>(&mut self, arg: S) -> &mut Command {
        self.args.push(arg.as_ref().to_string());
        self
    }

    pub fn stdin(&mut self, file: Arc<dyn FileLike>) -> &mut Command {
        self.stdin = Some(file);
        self
    }

    pub fn stdout(&mut self, file: Arc<dyn FileLike>) -> &mut Command {
        self.stdout = Some(file);
        self
    }

    pub fn port(&mut self, port: Port) -> &mut Command {
        self.ports.push(port);
        self
    }

    pub fn spawn(&mut self, tcpip_ch: Channel) -> Result<(), Error> {
        let mut builder = FileSystemBuilder::new();

        let command_json = serde_json::to_vec(&CommandJson {
            program: self.program.clone(),
            args: self.args.clone(),
        })
        .unwrap();

        builder.add_root_file("command", BufferedStdin::new(command_json));

        if let Some(stdout) = &self.stdout {
            builder.add_root_file("stdout", stdout.clone());
        }

        if let Some(stdin) = &self.stdin {
            builder.add_root_file("stdin", stdin.clone());
        }

        let rootfs_file = match &self.image {
            Some(ContainerImage::Static(data)) => Arc::new(RootfsProvider::from_static(data)),
            None => panic!("rootfs is not set"),
        };
        builder.add_root_file("rootfs", rootfs_file);

        let fs = builder.build();
        boot_linux(fs, &self.ports, tcpip_ch);
        Ok(())
    }
}
