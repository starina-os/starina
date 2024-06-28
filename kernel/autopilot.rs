use alloc::string::String;
use alloc::vec::Vec;

use ftl_autogen::protocols::autopilot::NewclientRequest;
use ftl_types::error::FtlError;
use ftl_types::handle::HandleRights;
use ftl_types::message::HandleOwnership;
use ftl_types::message::MessageBuffer;
use ftl_types::message::MessageSerialize;
use ftl_types::spec::AppSpec;
use hashbrown::HashMap;

use crate::app_loader::AppLoader;
use crate::channel::Channel;
use crate::cpuvar::current_thread;
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

    pub fn start_apps(&mut self, apps: Vec<(String, AppSpec, &'static [u8])>) -> Result<(), Error> {
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
            for name in &app.spec.depends {
                let proto_name = ProtocolName(name.clone());
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
                    handle: HandleOwnership(handle_id),
                })
                .serialize(&mut msgbuffer);

                provider
                    .our_ch
                    .send(NewclientRequest::MSGINFO, &msgbuffer)
                    .map_err(Error::SendMessage)?;

                depend_handles.push((
                    proto_name.0,
                    Handle::new(app_ch.into(), HandleRights::NONE).into(),
                ));
            }

            let their_ch_handle = Handle::new(
                self.their_chs.remove(&app.name).unwrap().into(),
                HandleRights::NONE,
            )
            .into();

            AppLoader::parse(app.elf_file)
                .expect("invalid ELF")
                .load(their_ch_handle, depend_handles)
                .expect("failed to load ELF");
        }

        Ok(())
    }
}
