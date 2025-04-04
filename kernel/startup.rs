use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use arrayvec::ArrayVec;
use hashbrown::HashMap;
use starina::device_tree::BusNode;
use starina::device_tree::DeviceTree;
use starina::handle::HandleId;
use starina::handle::HandleRights;
use starina::message::MessageInfo;
use starina::message::MessageKind;
use starina::spec::DeviceMatch;
use starina::spec::EnvType;
use starina::spec::ExportItem;
use starina::syscall::VsyscallPage;
use starina_types::spec::AppSpec;

use crate::channel::Channel;
use crate::handle::Handle;
use crate::iobus::NOMMU_IOBUS;
use crate::isolation::IsolationHeap;
use crate::process::KERNEL_PROCESS;
use crate::scheduler::GLOBAL_SCHEDULER;
use crate::spinlock::SpinLock;
use crate::thread::Thread;

struct InKernelApp {
    name: &'static str,
    main: fn(vsyscall: *const VsyscallPage),
    spec: AppSpec,
}

const INKERNEL_APPS: &[InKernelApp] = &[
    InKernelApp {
        name: "virtio_net",
        main: virtio_net::autogen::app_main,
        spec: virtio_net::autogen::APP_SPEC,
    },
    InKernelApp {
        name: "tcpip",
        main: tcpip::autogen::app_main,
        spec: tcpip::autogen::APP_SPEC,
    },
];

static INSTANCES: SpinLock<Vec<Instance>> = SpinLock::new(Vec::new());

struct Instance {
    vsyscall_page: Box<VsyscallPage>,
    environ_str: String,
}

pub fn load_inkernel_apps(device_tree: DeviceTree) {
    let mut instances = INSTANCES.lock();
    let mut server_channels = HashMap::new();
    let mut client_channels = HashMap::new();
    for app in INKERNEL_APPS {
        for export in app.spec.exports {
            match export {
                ExportItem::Service { name } => {
                    let (ch1, ch2) = Channel::new().unwrap();
                    assert!(
                        server_channels.insert(app.name, ch1).is_none(),
                        "multiple exports are not yet supported"
                    );
                    client_channels.insert(*name, ch2);
                }
            }
        }
    }

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
                EnvType::Service { name } => {
                    let ch = match client_channels.get(name) {
                        Some(ch) => ch.clone(),
                        None => panic!("service not found: {} (requested by {})", name, app.name),
                    };

                    // Enqueue a connect message to the server.
                    let (server_ch, client_ch) = Channel::new().unwrap();
                    {
                        let server_handles = KERNEL_PROCESS.handles();
                        let server_ch_handle =
                            Handle::new(server_ch, HandleRights::READ | HandleRights::WRITE);

                        let mut handles = ArrayVec::new();
                        handles.push(server_ch_handle.into());

                        ch.do_send(
                            MessageInfo::new(MessageKind::Connect as i32, 0, 1),
                            vec![],
                            handles,
                        )
                        .expect("failed to send connect message");
                    }

                    // Add the client channel to the environment.
                    let handle_id = {
                        let handles = KERNEL_PROCESS.handles();
                        let handle =
                            Handle::new(client_ch, HandleRights::READ | HandleRights::WRITE);
                        handles
                            .lock()
                            .insert(handle)
                            .expect("failed to insert channel")
                    };

                    serde_json::json!(handle_id.as_raw())
                }
            };

            env.insert(item.name.into(), value);
        }

        let startup_ch = if let Some(ch) = server_channels.get(app.name) {
            let handle = Handle::new(ch.clone(), HandleRights::READ | HandleRights::WRITE);
            let handle_id = KERNEL_PROCESS.handles().lock().insert(handle).unwrap();
            handle_id
        } else {
            HandleId::from_raw(0)
        };

        let env_str = serde_json::to_string(&env).unwrap();
        let vsyscall_page = Box::new(VsyscallPage {
            environ_ptr: env_str.as_ptr(),
            environ_len: env_str.len(),
            startup_ch,
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
