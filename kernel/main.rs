#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(test, feature(test))]
#![no_main]

extern crate alloc;

#[macro_use]
mod print;

mod allocator;
mod arch;
mod panic;
mod spinlock;

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::arch::halt;
use crate::spinlock::SpinLock;

pub struct App2;
impl starina::Worker for App2 {
    type Context = usize;
    fn init() -> Self {
        App2
    }
}

trait DynWorker {
    fn dyn_call(&self);
}

struct Instance<W: starina::Worker> {
    worker: W,
    ctx: W::Context,
}

impl<W: starina::Worker> Instance<W> {
    fn new(ctx: W::Context) -> Instance<W> {
        let worker = W::init();
        Self { worker, ctx }
    }
}

impl<W: starina::Worker> DynWorker for Instance<W> {
    fn dyn_call(&self) {
        self.worker.call(&self.ctx);
    }
}

#[repr(C)]
struct RawBuffer {
    data: [u8; 4096],
}
pub struct OwnedBuffer(*mut RawBuffer);
impl Drop for OwnedBuffer {
    fn drop(&mut self) {
        GLOBAL_POOL.lock().free(self.0);
    }
}

pub struct OwnedBufferPool {
    buffers: Vec<*mut RawBuffer>,
}
unsafe impl Sync for OwnedBufferPool {}

static GLOBAL_POOL: SpinLock<OwnedBufferPool> = SpinLock::new(OwnedBufferPool {
    buffers: Vec::new(),
});
impl OwnedBufferPool {
    fn free(&mut self, buffer: *mut RawBuffer) {
        self.buffers.push(buffer);
    }
    fn alloc(&mut self) -> OwnedBuffer {
        match self.buffers.pop() {
            Some(buffer) => OwnedBuffer(buffer),
            None => {
                let uninit = unsafe { core::mem::uninitialized() };
                OwnedBuffer(Box::into_raw(Box::new(uninit)))
            }
        }
    }
}

pub fn boot() -> ! {
    {
        use alloc::boxed::Box;

        let apps: [Box<dyn DynWorker>; 2] = [
            Box::new(Instance::<ktest::App>::new(())) as Box<dyn DynWorker>,
            Box::new(Instance::<App2>::new(0usize)) as Box<dyn DynWorker>,
        ];

        for app in apps {
            app.dyn_call();
        }
    }

    println!("\nBooting Starina...");
    halt();
}
