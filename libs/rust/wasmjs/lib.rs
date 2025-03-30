#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::convert::Infallible;

use wasmtime::Caller;
use wasmtime::Config;
use wasmtime::CustomCodeMemory;
use wasmtime::Engine;
use wasmtime::Extern;
use wasmtime::Func;
use wasmtime::Module;
use wasmtime::Store;
use wasmtime::Trap;

pub struct CodeMemoryAllocator {}

impl CodeMemoryAllocator {
    pub fn new() -> Self {
        Self {}
    }
}

impl CustomCodeMemory for CodeMemoryAllocator {
    fn required_alignment(&self) -> usize {
        align_of::<u8>()
    }

    fn publish_executable(&self, _ptr: *const u8, _size: usize) -> anyhow::Result<()> {
        Ok(())
    }

    fn unpublish_executable(&self, _ptr: *const u8, _size: usize) -> anyhow::Result<()> {
        Ok(())
    }
}
pub fn test(console_write: extern "C" fn(*const u8, usize)) {
    let mut config = Config::new();
    config.memory_reservation(1024 * 1024);
    config.with_custom_code_memory(Some(Arc::new(CodeMemoryAllocator::new())));

    let engine = Engine::new(&config).unwrap();
    let module =
        unsafe { Module::deserialize(&engine, include_bytes!("app.precompiled")).unwrap() };

    let mut store = Store::new(&engine, ());

    let answer = Func::wrap(&mut store, move |caller: Caller<'_, ()>, x: i32| {
        let s = format!("{}", x);
        console_write(s.as_ptr(), s.len());
        Ok(())
    });

    let instance = Instance::new(&mut store, &module, &[answer]).unwrap();
    let entrypoint = instance.get_typed_func::<(), ()>(&mut store, "foo")?;
    entrypoint.call(&mut store, ())?;
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_longjmp(a: *mut i8, b: i32) -> i32 {
    panic!("longjmp called");
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_tls_get() -> *mut u8 {
    panic!("wasmtime_tls_get called");
}
