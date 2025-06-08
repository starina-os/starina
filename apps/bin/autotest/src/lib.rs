#![no_std]

use core::time::Duration;

use starina::handle::Handleable;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::*;
use starina::spec::AppSpec;
use starina::timer::Timer;

pub const SPEC: AppSpec = AppSpec {
    name: "autotest",
    env: &[],
    exports: &[],
    main,
};

fn main(_env_json: &[u8]) {
    info!("Starting timer tests - watch for changing intervals!");

    test_dynamic_timers();
}

fn test_dynamic_timers() {
    let timer = Timer::new().expect("failed to create timer");
    let poll: Poll<()> = Poll::new().expect("failed to create poll");

    poll.add(timer.handle_id(), (), Readiness::READABLE)
        .expect("failed to add timer to poll");

    // Test sequence with different intervals
    let intervals = [
        (Duration::from_millis(200), "Fast (200ms)"),
        (Duration::from_millis(500), "Medium (500ms)"),
        (Duration::from_millis(1000), "Slow (1s)"),
    ];

    let mut cycle_count = 0;
    let mut interval_index = 0;
    let ticks_per_interval = 5; // Number of ticks before changing interval

    info!(
        "Timer test pattern: Each interval will tick {} times",
        ticks_per_interval
    );

    loop {
        let (current_interval, interval_name) = intervals[interval_index];

        if cycle_count % ticks_per_interval == 0 {
            info!("Switching to {} interval", interval_name);
        }

        timer
            .set_timeout(current_interval)
            .expect("failed to set timer");

        poll.wait().expect("failed to wait on poll");

        let tick_in_cycle = (cycle_count % ticks_per_interval) + 1;
        info!(
            "TICK #{} at {} ({}/{})",
            cycle_count + 1,
            interval_name,
            tick_in_cycle,
            ticks_per_interval
        );

        cycle_count += 1;

        // Change interval every ticks_per_interval ticks
        if cycle_count % ticks_per_interval == 0 {
            interval_index = (interval_index + 1) % intervals.len();

            if interval_index == 0 {
                info!("=== Completed full cycle, starting over ===");
            }
        }
    }
}
