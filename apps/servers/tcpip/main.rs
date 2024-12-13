#![no_std]
#![no_main]
#![allow(unused)]

use core::arch::global_asm;
use core::cell::RefCell;

use starina::info;
use starina::mainloop::Event;
use starina::mainloop::Mainloop;
use starina::warn;

#[derive(Debug)]
enum Context {
    Control,
    TcpConn,
}

global_asm!(r#"
.global use_it
use_it:
    msr tpidr_el0, x0
    ret
"#);

extern "C" {
    fn use_it(value: u32);
}

#[no_mangle]
pub fn main() {
    let mut mainloop = Mainloop::<Context>::new().unwrap();
    let refcell = RefCell::new(0);
    mainloop.run(|ev| {
        match ev {
            Event::Message {
                ctx,
                message: starina_types::message::Message::Ping(starina_types::message::Ping { value2, .. }),
               sender,
                ..
            } => {
                unsafe {
                    use_it(value2);
                }
            }
            _ => {
                warn!("unknown message");
            }
            Event::Error(err) => {
                panic!("err: {:?}", err);
            }
        }
    });
}
