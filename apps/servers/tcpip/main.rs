#![no_std]
#![no_main]
#![allow(unused)]

use core::cell::RefCell;

use starina::info;
use starina::mainloop::Event;
use starina::mainloop::Mainloop;

#[derive(Debug)]
enum Context {
    Control,
    TcpConn,
}

#[no_mangle]
pub fn main() {
    let mut mainloop = Mainloop::<Context>::new().unwrap();
    let refcell = RefCell::new(0);
    mainloop.run(|ev| {
        match ev {
            Event::Message {
                ctx,
                message,
                sender,
                ..
            } => {}
            Event::Error(err) => {
                panic!("err: {:?}", err);
            }
        }
    });
}
