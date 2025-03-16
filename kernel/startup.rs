use alloc::string::String;
use alloc::vec::Vec;

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
    environ_str: String,
}

pub fn load_inkernel_apps() {
    let mut instances = INSTANCES.lock();
    let device_tree_json = serde_json::json!({});
    for app in INKERNEL_APPS {
        let mut items = Vec::with_capacity(app.spec.env.len());
        for item in app.spec.env {
            items.push(match item.ty {
                EnvType::DeviceTree {} => {
                    serde_json::json!({
                        "name": item.name,
                        "type": "device_tree",
                        "value": device_tree_json,
                    })
                }
            });
        }

        let env_json = serde_json::json!({
            "env": items,
        });
        let env_str = serde_json::to_string(&env_json).unwrap();

        let thread = Thread::new_inkernel(app.main as usize, 0).unwrap();
        GLOBAL_SCHEDULER.push(thread);
        instances.push(Instance {
            environ_str: env_str,
        });
    }
}
