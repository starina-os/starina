use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use hashbrown::HashMap;
use starina::device_tree::BusNode;
use starina::device_tree::DeviceTree;
use starina::handle::HandleRights;
use starina::spec::DeviceMatch;
use starina::spec::EnvType;
use starina::syscall::VsyscallPage;
use starina_types::spec::AppSpec;

use crate::handle::Handle;
use crate::iobus::NOMMU_IOBUS;
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
    for app in INKERNEL_APPS {
        info!("startup: starting \"{}\"", app.name);
        let mut env = serde_json::Map::new();
        for item in app.spec.env {
            let value = match item.ty {
                EnvType::IoBusMap => {
                    let mut buses = HashMap::new();
                    for (name, node) in &device_tree.buses {
                        let bus = match node {
                            BusNode::NoMmu => NOMMU_IOBUS.clone(),
                        };

                        let handle = Handle::new(bus, HandleRights::WRITE);
                        let handle_id = KERNEL_PROCESS
                            .handles()
                            .lock()
                            .insert(handle)
                            .expect("failed to insert iobus");
                        buses.insert(name, handle_id.as_raw());
                    }

                    serde_json::json!(buses)
                }
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

                    serde_json::json!({
                        "devices": devices,
                        "buses": device_tree.buses,
                    })
                }
            };

            env.insert(item.name.into(), value);
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
