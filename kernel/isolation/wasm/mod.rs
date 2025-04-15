use alloc::boxed::Box;

use wasmi::CompilationMode;
use wasmi::Config;
use wasmi::Engine;
use wasmi::Instance;
use wasmi::Linker;
use wasmi::Module;
use wasmi::Store;

mod wasi;

pub(super) struct HostState {}

impl HostState {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct Runner {
    store: Store<HostState>,
    instance: Instance,
}

impl Runner {
    pub fn init(wasm: &[u8]) -> Result<Self, wasmi::Error> {
        let mut config = Config::default();
        config.compilation_mode(CompilationMode::Lazy);
        config.wasm_bulk_memory(true);

        let engine = Engine::new(&config);
        // FIXME: Do not use new_unchecked.
        let module = unsafe { Module::new_unchecked(&engine, wasm) }?;
        let state = HostState::new();
        let mut store = Store::new(&engine, state);
        let mut linker = <Linker<HostState>>::new(&engine);

        wasi::link_wasi(&mut linker);

        let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;
        Ok(Runner { store, instance })
    }

    pub fn run(mut self) {
        let start_func = self
            .instance
            .get_typed_func::<(), ()>(&self.store, "_start")
            .expect("failed to get _start");

        start_func.call(&mut self.store, ()).unwrap();
    }
}

pub extern "C" fn app_entrypoint(runner_ptr: *mut Runner) {
    let mut runner = unsafe { Box::from_raw(runner_ptr) };
    runner.run();
    panic!("WASM app exited");
}
