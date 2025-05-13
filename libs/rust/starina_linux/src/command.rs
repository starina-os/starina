use starina::prelude::*;
use starina::sync::Arc;

use crate::boot::boot_linux;
use crate::fs::Entry;
use crate::fs::FileSystem;

#[derive(Debug)]
pub enum Error {}

pub struct Command {
    pub argv: Vec<String>,
    pub stdin: Option<Arc<dyn Entry>>,
    pub stdout: Option<Arc<dyn Entry>>,
}

impl Command {
    pub fn new(program: &str) -> Self {
        Command {
            argv: vec![program.to_string()],
            stdin: None,
            stdout: None,
        }
    }

    pub fn arg<S: AsRef<str>>(&mut self, arg: S) -> &mut Self {
        self.argv.push(arg.as_ref().to_string());
        self
    }

    pub fn stdin(&mut self, file: Arc<dyn Entry>) -> &mut Self {
        self.stdin = Some(file);
        self
    }

    pub fn stdout(&mut self, file: Arc<dyn Entry>) -> &mut Self {
        self.stdout = Some(file);
        self
    }

    pub fn spawn(self) -> Result<(), Error> {
        let mut root_files = Vec::new();
        if let Some(stdin) = self.stdin {
            root_files.push(("stdin", stdin));
        }

        let fs = FileSystem::new(root_files);
        boot_linux(fs);
        Ok(())
    }
}
