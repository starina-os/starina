#![no_std]

use starina::prelude::*;
use starina::spec::AppSpec;

pub const APP_SPEC: AppSpec = AppSpec {
    name: "autotest",
    env: &[],
    exports: &[],
    main,
};

fn main(_env_json: &[u8]) {
    info!("running automated tests...");
    // Thread::spawn(|| {
    //     for i in 0.. {
    //         if i % 100000 == 0 {
    //             starina::syscall::console_write(b"A");
    //         }
    //     }
    // })
    // .unwrap();
    // Thread::spawn(|| {
    //     for i in 0.. {
    //         if i % 100000 == 0 {
    //             starina::syscall::console_write(b"B");
    //         }
    //     }
    // })
    // .unwrap();

    // for i in 0.. {
    //     if i % 100000 == 0 {
    //         starina::syscall::console_write(b"C");
    //     }
    // }
}
