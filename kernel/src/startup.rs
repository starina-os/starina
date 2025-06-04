use alloc::boxed::Box;
use alloc::vec;

use arrayvec::ArrayVec;
use hashbrown::HashMap;
use starina::device_tree::DeviceTree;
use starina::handle::HandleRights;
use starina::message::MessageInfo;
use starina::message::MessageKind;
use starina::spec::AppSpec;
use starina::spec::DeviceMatch;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::spec::ExportItem;
use starina::syscall::VsyscallPage;

use crate::channel::Channel;
use crate::handle::Handle;
use crate::process::KERNEL_PROCESS;
use crate::scheduler::GLOBAL_SCHEDULER;
use crate::thread::Thread;

const INKERNEL_APPS: &[AppSpec] = &[
    autotest::SPEC,
    virtio_net::SPEC,
    tcpip::SPEC,
    http_server::SPEC,
    // catsay::autogen::APP_SPEC,
];

pub fn load_inkernel_apps(device_tree: DeviceTree) {
    let mut server_channels = HashMap::new();
    let mut client_channels = HashMap::new();
    for spec in INKERNEL_APPS {
        for export in spec.exports {
            match export {
                ExportItem::Service { service: name } => {
                    let (ch1, ch2) = Channel::new().unwrap();
                    assert!(
                        server_channels.insert(spec.name, ch1).is_none(),
                        "multiple exports are not yet supported"
                    );
                    client_channels.insert(*name, ch2);
                }
            }
        }
    }

    for spec in INKERNEL_APPS {
        info!("startup: starting \"{}\"", spec.name);
        let mut env = serde_json::Map::new();
        for EnvItem { name: env_name, ty } in spec.env {
            let value = match ty {
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
                    })
                }
                EnvType::Service { service: name } => {
                    let ch = match client_channels.get(name) {
                        Some(ch) => ch.clone(),
                        None => panic!("service not found: {} (requested by {})", name, spec.name),
                    };

                    // Enqueue a connect message to the server.
                    let (server_ch, client_ch) = Channel::new().unwrap();
                    {
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

            env.insert((*env_name).into(), value);
        }

        if let Some(ch) = server_channels.get(spec.name) {
            let handle = Handle::new(ch.clone(), HandleRights::READ | HandleRights::WRITE);
            let handle_id = KERNEL_PROCESS.handles().lock().insert(handle).unwrap();
            env.insert("startup_ch".into(), serde_json::json!(handle_id.as_raw()));
        };

        let env_str = serde_json::to_string(&env).unwrap();
        let vsyscall_page = Box::new(VsyscallPage {
            environ_ptr: env_str.as_ptr(),
            environ_len: env_str.len(),
            name: spec.name.as_ptr(),
            name_len: spec.name.len(),
            main: spec.main,
        });

        let arg = &*vsyscall_page as *const VsyscallPage as usize;
        let thread = Thread::new_inkernel(starina::start::start as usize, arg as usize).unwrap();

        GLOBAL_SCHEDULER.push(thread);
    }
}
