use crate::arch::halt;

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
