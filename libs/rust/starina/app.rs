use crate::poll::Poll;
use crate::syscall;

pub trait App: Send + Sync {
    fn init() -> Self
    where
        Self: Sized;

    fn tick(&mut self);
}

pub struct Dispatcher {
    poll: Poll,
}

impl Dispatcher {
    pub fn new(poll: Poll) -> Self {
        Self { poll }
    }
}

pub fn app_loop(app: impl App) {
    let poll = Poll::create().unwrap();
    let dispatcher = Dispatcher::new(poll);
    loop {
        // let ev = syscall::poll_wait(poll).unwrap();
    }
}
