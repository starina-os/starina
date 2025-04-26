//! Thread-local storage.

use alloc::boxed::Box;
use core::arch::asm;

pub struct Storage {
    pub name: &'static str,
}

fn read_register() -> usize {
    if cfg!(target_arch = "riscv64") {
        let mut value: usize;
        unsafe {
            asm!("mv {}, tp", out(reg) value);
        }
        value
    } else {
        unimplemented!();
    }
}

fn set_register(value: usize) {
    if cfg!(target_arch = "riscv64") {
        unsafe {
            asm!("mv tp, {}", in(reg) value);
        }
    } else {
        unimplemented!();
    }
}

pub fn thread_local() -> &'static Storage {
    let reg = read_register();
    let ptr = reg as *const Storage;
    debug_assert!(!ptr.is_null());
    unsafe { &*ptr }
}

pub(crate) fn init_thread_local(name: &'static str) {
    debug_assert_eq!(read_register(), 0, "thread local storage already set");

    let storage = Box::leak(Box::new(Storage { name }));
    set_register(storage as *const _ as usize);
}
