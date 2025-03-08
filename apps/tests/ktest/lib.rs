#![no_std]
use starina::app::App;
use starina::app::Dispatcher;
use starina::channel::userspace::Channel;
use starina::channel::userspace::message::PingReader;
use starina::channel::userspace::message::PingWriter;
use starina::handle::Handleable;
use starina::info;

pub struct Main {}

impl App for Main {
    fn init(dispatcher: &Dispatcher, ch: Channel) -> Self {
        info!("Hello from ktest!");
        ch.send(PingWriter { value: 0 }).unwrap();
        dispatcher.add_channel(ch).unwrap();
        info!("added channel");
        Main {}
    }

    fn on_ping(&self, ch: &Channel, ping: PingReader) {
        info!("ktest: ch={:?}: ping={}", ch.handle_id(), ping.value());
        ch.send(PingWriter {
            value: ping.value() + 1,
        })
        .unwrap();
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
