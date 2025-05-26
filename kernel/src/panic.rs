use core::panic::PanicInfo;
use core::sync::atomic::AtomicU8;
use core::sync::atomic::Ordering;


use crate::arch;

/// Panic counter. Every time the kernel panics, this counter is incremented.
static PANIC_COUNTER: AtomicU8 = AtomicU8::new(0);

/// Kernel panic handler.
#[cfg_attr(target_os = "none", panic_handler)]
#[cfg_attr(not(target_os = "none"), allow(unused))]
fn panic(info: &PanicInfo) -> ! {
    // In case it panics while handling a panic, this panic handler implements
    // some fallback logic to try to at least print the panic details.
    match PANIC_COUNTER.fetch_add(1, Ordering::SeqCst) {
        0 => {
            // First panic: Try whatever we can do including complicated stuff
            // which may panic again.
            error!("kernel panic: {}", info);
            arch::halt();
        }
        1 => {
            // Double panics: paniked while handling a panic. Keep it simple.
            println!("double kernel panic: {:?}", info);
            arch::halt();
        }
        _ => {
            // Triple panics: println! seems to be broken. Spin forever.
            arch::halt();
        }
    }
}
