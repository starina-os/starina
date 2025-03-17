use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use hashbrown::HashMap;
use starina::device_tree::DeviceTree;
use starina::spec::DeviceMatch;
use starina::spec::EnvType;
use starina::syscall::VsyscallPage;
use starina_types::spec::AppSpec;

use crate::process::KERNEL_PROCESS;
use crate::scheduler::GLOBAL_SCHEDULER;
use crate::spinlock::SpinLock;
use crate::thread::Thread;

struct InKernelApp {
    name: &'static str,
    main: fn(vsyscall: *const VsyscallPage),
    spec: AppSpec,
}

const INKERNEL_APPS: &[InKernelApp] = &[InKernelApp {
    name: "virtio_net",
    main: virtio_net::autogen::app_main,
    spec: virtio_net::autogen::APP_SPEC,
}];

static INSTANCES: SpinLock<Vec<Instance>> = SpinLock::new(Vec::new());

struct Instance {
    vsyscall_page: Box<VsyscallPage>,
    environ_str: String,
}

pub fn load_inkernel_apps(device_tree: DeviceTree) {
    let mut instances = INSTANCES.lock();
    let mut env = serde_json::Map::new();
    for app in INKERNEL_APPS {
        info!("startup: loading app {}", app.name);
        let mut items = Vec::with_capacity(app.spec.env.len());
        for item in app.spec.env {
            items.push(match item.ty {
                EnvType::DeviceTree { matches } => {
                    let mut devices = HashMap::new();
                    for (name, node) in &device_tree.devices {
                        let should_add = matches.iter().any(|m| {
                            match m {
                                DeviceMatch::Compatible(compatible) => {
                                    node.compatible.iter().any(|c| c == compatible)
                                }
                            }
                        });

                        if should_add {
                            devices.insert(name, node);
                        }
                    }

                    env.insert(
                        item.name.into(),
                        serde_json::json!({
                            "buses": device_tree.buses,
                            "devices": devices,
                        }),
                    );
                }
            });
        }

        let env_str = serde_json::to_string(&env).unwrap();
        let vsyscall_page = Box::new(VsyscallPage {
            environ_ptr: env_str.as_ptr(),
            environ_len: env_str.len(),
        });

        let arg = unsafe { &*vsyscall_page as *const VsyscallPage } as usize;
        let thread = Thread::new_inkernel(app.main as usize, arg as usize).unwrap();
        GLOBAL_SCHEDULER.push(thread);
        instances.push(Instance {
            vsyscall_page,
            environ_str: env_str,
        });
    }
}
