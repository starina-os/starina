#![no_std]
use starina::app::App;
use starina::app::Dispatcher;
use starina::channel::userspace::Channel;
use starina::channel::userspace::message::Ping;
use starina::info;

pub struct Main {}

impl App for Main {
    fn init(dispatcher: &Dispatcher, ch: Channel) -> Self {
        info!("Hello from ktest!");
        dispatcher.add_channel(ch).unwrap();
        Main {}
    }

    fn on_ping(&self, ch: &Channel, ping: Ping) {
        info!("ktest: ping={}", ping.value());
        todo!()
    }
}

// TODO: Remove this.
pub fn app_main(handle_id: isize) {
    use starina::handle::HandleId;
    use starina::handle::OwnedHandle;

    let handle_id = HandleId::from_raw(handle_id.try_into().unwrap());
    let handle = OwnedHandle::from_raw(handle_id);
    let ch = Channel::from_handle(handle);
    starina::app::app_loop::<Main>(ch);
}
