#![no_std]
#![no_main]
#![allow(unused)]

use starina::{info, mainloop::{Event, Mainloop}};

#[derive(Debug)]
enum Context {
    Control,
}

#[no_mangle]
pub fn main() {
    let mut mainloop = Mainloop::<Context>::new().unwrap();
    loop {
        let ev = mainloop.next();
        match ev {
            Event::Message { ctx, message, sender, .. } => {
                match ctx {
                    Context::Control => {
                        info!("message: {:?}", message);
                    }
                }
            }
            Event::Error(err) => {
                panic!("err: {:?}", err);
            }
        }
    }
}
