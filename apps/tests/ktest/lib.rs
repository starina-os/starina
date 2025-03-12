#![no_std]
use starina::app::App;
use starina::app::Context;
use starina::app::Dispatcher;
use starina::channel::AnyMessage;
use starina::channel::Channel;
use starina::channel::message::PingReader;
use starina::channel::message::PingWriter;
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

    fn on_message(&self, ctx: &Context, msg: AnyMessage) {
        let ping = PingReader::try_from(msg).unwrap();
        info!("ktest: ch={:?}: ping={}", ctx.sender.handle(), ping.value());
        ctx.sender
            .send(PingWriter {
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
