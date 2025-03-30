#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::convert::Infallible;
use core::mem::MaybeUninit;

use wasmtime::Caller;
use wasmtime::Config;
use wasmtime::CustomCodeMemory;
use wasmtime::Engine;
use wasmtime::Extern;
use wasmtime::Func;
use wasmtime::Instance;
use wasmtime::Module;
use wasmtime::Store;
use wasmtime::Trap;

pub fn test(console_write: fn(&[u8])) {
    let mut config = Config::new();
    config.memory_reservation(1024 * 1024);

    let engine = Engine::new(&config).unwrap();
    let module =
        unsafe { Module::deserialize(&engine, include_bytes!("app.precompiled")).unwrap() };

    let mut store = Store::new(&engine, ());

    let answer = Func::wrap(&mut store, move |caller: Caller<'_, ()>, x: i32| {
        let s = alloc::format!("{}", x);
        console_write(s.as_bytes());
        Ok(())
    });

    let instance = Instance::new(&mut store, &module, &[Extern::Func(answer)]).unwrap();
    let entrypoint = instance
        .get_typed_func::<(), ()>(&mut store, "main")
        .unwrap();
    entrypoint.call(&mut store, ()).unwrap();
}

/// ```c
/// int wasmtime_mmap_new(uintptr_t size, uint32_t prot_flags, uint8_t **ret);
/// ```
#[unsafe(no_mangle)]
pub fn wasmtime_mmap_new(size: usize, prot_flags: u32, ret: *mut *mut u8) -> i32 {
    let layout = Layout::from_size_align(size, 4096).unwrap();
    let ptr = unsafe { alloc::alloc::alloc(layout) };
    unsafe {
        *ret = ptr;
    }
    0
}

#[unsafe(no_mangle)]
pub fn wasmtime_memory_image_new() {
    panic!("wasmtime_memory_image_new called");
}

#[unsafe(no_mangle)]
pub fn wasmtime_memory_image_free() {
    panic!("wasmtime_memory_image_free called");
}

#[unsafe(no_mangle)]
pub fn wasmtime_memory_image_map_at() -> usize {
    panic!("wasmtime_memory_image_map_at called");
}

#[unsafe(no_mangle)]
pub fn wasmtime_page_size() -> usize {
    return 4096;
}

#[unsafe(no_mangle)]
pub fn wasmtime_mprotect() -> usize {
    panic!("wasmtime_mprotect called");
}

#[unsafe(no_mangle)]
pub fn wasmtime_mmap_remap() -> usize {
    panic!("wasmtime_mmap_remap called");
}

#[unsafe(no_mangle)]
pub fn wasmtime_munmap(ptr: *mut u8, size: usize) -> usize {
    let layout = Layout::from_size_align(size, 4096).unwrap();
    unsafe {
        alloc::alloc::dealloc(ptr, layout);
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_setjmp(a: *mut i8) -> i32 {
    panic!("setjmp called");
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_longjmp(a: *mut i8, b: i32) -> i32 {
    panic!("longjmp called");
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_tls_get() -> *mut u8 {
    panic!("wasmtime_tls_get called");
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_tls_set(a: *mut u8) {
    panic!("wasmtime_tls_set called");
}
