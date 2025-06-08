#![no_std]

use starina::prelude::*;
use starina::spec::AppSpec;
use starina::timer::Timer;
use starina::poll::{Poll, Readiness};

pub const SPEC: AppSpec = AppSpec {
    name: "autotest",
    env: &[],
    exports: &[],
    main,
};

fn main(_env_json: &[u8]) {
    info!("running automated tests...");
    
    let timer = Timer::new().expect("failed to create timer");
    let poll: Poll<()> = Poll::new().expect("failed to create poll");
    
    poll.add(timer.handle(), (), Readiness::READABLE).expect("failed to add timer to poll");
    
    loop {
        timer.set_timeout_ms(1000).expect("failed to set timer");
        
        poll.wait().expect("failed to wait on poll");
        
        info!("hello");
    }
}
