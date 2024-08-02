use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

use ftl_autogen::protocols::autopilot::NewclientRequest;
use ftl_types::error::FtlError;
use ftl_types::handle::HandleRights;
use ftl_types::message::MessageBuffer;
use ftl_types::message::MessageSerialize;
use ftl_types::message::MovedHandle;
use ftl_types::spec::AppSpec;
use ftl_types::spec::BootSpec;
use ftl_types::spec::Depend;
use ftl_types::spec::Spec;
use hashbrown::HashMap;

use crate::app_loader::AppLoader;
use crate::bootfs::Bootfs;
use crate::channel::Channel;
use crate::cpuvar::current_thread;
use crate::device_tree;
use crate::device_tree::DeviceTree;
use crate::handle::Handle;
use crate::ref_counted::SharedRef;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppName(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProtocolName(String);

struct App {
    name: AppName,
    spec: AppSpec,
    elf_file: &'static [u8],
    our_ch: SharedRef<Channel>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    CreateChannel(FtlError),
    SendMessage(FtlError),
    NoService {
        app: AppName,
        protocol: ProtocolName,
    },
}

pub struct Autopilot {
    providers: HashMap<ProtocolName, AppName>,
    their_chs: HashMap<AppName, SharedRef<Channel>>,
    apps: HashMap<AppName, App>,
}

impl Autopilot {
    pub fn new() -> Autopilot {
        Autopilot {
            providers: HashMap::new(),
            their_chs: HashMap::new(),
            apps: HashMap::new(),
        }
    }

    pub fn boot(&mut self, bootfs: &Bootfs, boot_spec: &BootSpec, device_tree: &DeviceTree) {
        let mut apps = Vec::new();
        for app_name in &boot_spec.autostart_apps {
            let elf_file = match bootfs.find_by_name(&format!("apps/{}/app.elf", app_name)) {
                Some(file) => file,
                None => {
                    panic!("app.elf not found for \"{}\"", app_name);
                }
            };

            let spec_file = match bootfs.find_by_name(&format!("apps/{}/app.spec.json", app_name)) {
                Some(file) => file,
                None => {
                    panic!("app.spec.json not found for \"{}\"", app_name);
                }
            };

            let app_spec =
                match serde_json::from_slice(spec_file.data).expect("failed to parse spec") {
                    Spec::App(spec) => spec,
                    _ => panic!("unexpected spec type for \"{}\"", app_name),
                };

            apps.push((app_name.clone(), app_spec, elf_file.data));
        }

        self.start_apps(apps, device_tree.devices())
            .expect("failed to start apps");
    }

    fn start_apps(
        &mut self,
        apps: Vec<(String, AppSpec, &'static [u8])>,
        device_tree_devices: &[device_tree::Device],
    ) -> Result<(), Error> {
        for (name, spec, elf_file) in apps {
            let app_name = AppName(name.clone());

            let (their_ch, our_ch) = Channel::new().map_err(Error::CreateChannel)?;
            self.their_chs.insert(app_name.clone(), their_ch);

            self.apps.insert(
                app_name.clone(),
                App {
                    name: app_name,
                    spec: spec,
                    elf_file,
                    our_ch,
                },
            );
        }

        for app in self.apps.values() {
            for name in &app.spec.provides {
                self.providers
                    .insert(ProtocolName(name.clone()), app.name.clone());
            }
        }

        let current_thread = current_thread();
        let mut msgbuffer = MessageBuffer::new();
        for app in self.apps.values() {
            let mut depend_handles = Vec::new();
            let mut devices = Vec::new();
            for dep in &app.spec.depends {
                match &dep.depend {
                    Depend::Device { device_tree } => {
                        let mut found = Vec::new();
                        for device in device_tree_devices {
                            if let Some(device_tree) = &device_tree {
                                if device_tree
                                    .compatible
                                    .iter()
                                    .any(|compatible| compatible == device.compatible)
                                {
                                    let interrupts = match &device.interrupts {
                                        Some(interrupts) => {
                                            let mut vec = Vec::new();
                                            for interrupt in interrupts.iter() {
                                                vec.push(*interrupt);
                                            }
                                            Some(vec)
                                        }
                                        None => None,
                                    };

                                    found.push(ftl_types::environ::Device {
                                        name: device.name.to_string(),
                                        compatible: device.compatible.to_string(),
                                        reg: device.reg,
                                        interrupts,
                                    });
                                }
                            }
                        }

                        if found.is_empty() {
                            panic!("no device found for {:?}", device_tree);
                        }

                        devices.push((dep.name.clone(), found));
                    }
                    Depend::Service { protocol } => {
                        let proto_name = ProtocolName(protocol.clone());
                        let (provider_ch, app_ch) = Channel::new().map_err(Error::CreateChannel)?;
                        let provider_name = match self.providers.get(&proto_name) {
                            Some(name) => name,
                            None => {
                                return Err(Error::NoService {
                                    app: app.name.clone(),
                                    protocol: proto_name,
                                });
                            }
                        };

                        let handle_id = current_thread
                            .process()
                            .handles()
                            .lock()
                            .add(Handle::new(provider_ch.into(), HandleRights::NONE))
                            .unwrap();

                        let provider = self.apps.get(provider_name).unwrap();
                        (NewclientRequest {
                            handle: MovedHandle(handle_id),
                        })
                        .serialize(&mut msgbuffer);

                        provider
                            .our_ch
                            .send(NewclientRequest::MSGINFO, &msgbuffer)
                            .map_err(Error::SendMessage)?;

                        depend_handles.push((
                            dep.name.clone(),
                            Handle::new(app_ch.into(), HandleRights::NONE).into(),
                        ));
                    }
                }
            }

            let their_ch_handle = Handle::new(
                self.their_chs.remove(&app.name).unwrap().into(),
                HandleRights::NONE,
            )
            .into();

            AppLoader::parse(app.elf_file)
                .expect("invalid ELF")
                .load(their_ch_handle, depend_handles, devices)
                .expect("failed to load ELF");
        }

        Ok(())
    }
}
