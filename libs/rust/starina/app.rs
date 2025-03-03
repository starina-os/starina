use crate::syscall;

pub trait App: Send + Sync {
    fn init() -> Self
    where
        Self: Sized;

    fn tick(&mut self);
}

struct Poll;

pub struct Dispatcher {
    poll: Poll,
}

impl Dispatcher {
    pub fn new(poll: Poll) -> Self {
        Self { poll }
    }
}

pub fn app_loop(app: impl App) {
    let poll = syscall::poll_create().unwrap();
    let dispatcher = Dispatcher::new(todo!());
    loop {
        let ev = syscall::poll_wait(poll).unwrap();
    }
}
