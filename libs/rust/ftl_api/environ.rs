use alloc::vec::Vec;

use ftl_types::environ::Device;
use ftl_types::environ::EnvType;
use ftl_types::environ::EnvironDeserializer;
use ftl_types::handle::HandleId;
use hashbrown::HashMap;

use crate::channel::Channel;
use crate::handle::OwnedHandle;
use crate::vmspace::VmSpace;

#[derive(Debug)]
enum Value {
    Channel(Channel),
    VmSpace(VmSpace),
    Devices(Vec<Device>),
}

#[derive(Debug)]
pub struct Environ {
    entries: HashMap<&'static str, Value>,
}

impl Environ {
    pub fn parse(raw: &'static str) -> Environ {
        let mut entries = HashMap::new();
        let mut deserializer = EnvironDeserializer::new(raw);
        while let Some((key, env_type, value_str)) = deserializer.pop() {
            let value = match env_type {
                EnvType::Channel => {
                    let raw_handle_id = value_str.parse::<i32>().unwrap();
                    debug_assert!(raw_handle_id >= 0);

                    let handle_id = HandleId::from_raw(raw_handle_id);
                    let channel = Channel::from_handle(OwnedHandle::from_raw(handle_id));
                    Value::Channel(channel)
                }
                EnvType::VmSpace => {
                    let raw_handle_id = value_str.parse::<i32>().unwrap();
                    debug_assert!(raw_handle_id >= 0);

                    let handle_id = HandleId::from_raw(raw_handle_id);
                    let vmspace = VmSpace::from_handle(OwnedHandle::from_raw(handle_id));
                    Value::VmSpace(vmspace)
                }
                EnvType::Devices => {
                    let devices = serde_json::from_str(value_str).unwrap();
                    Value::Devices(devices)
                }
            };

            entries.insert(key, value);
        }

        Environ { entries }
    }

    pub fn take_channel(&mut self, key: &str) -> Option<Channel> {
        match self.entries.remove(key) {
            Some(Value::Channel(channel)) => Some(channel),
            Some(_) => panic!("not a channel"),
            None => None,
        }
    }

    pub fn take_vmspace(&mut self, key: &str) -> Option<VmSpace> {
        match self.entries.remove(key) {
            Some(Value::VmSpace(vmspace)) => Some(vmspace),
            Some(_) => panic!("not a channel"),
            None => None,
        }
    }

    pub fn devices(&self, key: &str) -> Option<&[Device]> {
        match self.entries.get(key) {
            Some(Value::Devices(devices)) => Some(devices),
            Some(_) => panic!("not devices"),
            None => None,
        }
    }
}
