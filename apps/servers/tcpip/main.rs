#![no_std]
#![no_main]
#![allow(unused)]

use core::cell::RefCell;

use starina::{info, mainloop::{Event, Mainloop}};

#[derive(Debug)]
enum Context {
    Control,
}

#[no_mangle]
pub fn main() {
    let mut mainloop = Mainloop::<Context>::new().unwrap();
    let refcell = RefCell::new(0);
    mainloop.run(|ev| {
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
    });
}
