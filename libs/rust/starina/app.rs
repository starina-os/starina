use crate::handle::HandleRights;
use crate::syscall;

pub trait App: Send + Sync {
    fn init() -> Self
    where
        Self: Sized;

    fn tick(&mut self);
}

pub struct Dispatcher {
    poll: HandleId,
}

impl Dispatcher {
    pub fn new(poll: HandleId) -> Self {
        Self { poll }
    }
}

fn app_loop() {
    let poll = syscall::poll_create().unwrap();
    let dispatcher = Dispatcher::new(poll);
    loop {
        let ev = syscall::poll_wait(poll).unwrap();
    }
}
