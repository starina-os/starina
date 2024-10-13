use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::str;
use core::str::Lines;

use serde::Deserialize;
use serde::Serialize;

use crate::handle::HandleId;

#[derive(PartialEq, Eq, Deserialize, Serialize)]
pub struct Device {
    pub name: String,
    pub compatible: String,
    pub reg: u64,
    pub interrupts: Option<Vec<u32>>,
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Device")
            .field("name", &self.name)
            .field("compatible", &self.compatible)
            .field("reg", &format_args!("{:#08x}", self.reg))
            .field("interrupts", &self.interrupts)
            .finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum EnvType {
    Channel,
    Devices,
    VmSpace,
    String,
}

pub struct EnvironSerializer(String);

impl EnvironSerializer {
    pub fn new() -> EnvironSerializer {
        EnvironSerializer(String::new())
    }

    pub fn finish(self) -> String {
        self.0
    }

    pub fn push<V>(&mut self, key: &str, env_type: EnvType, value: V)
    where
        V: fmt::Display,
    {
        use core::fmt::Write;

        debug_assert!(!key.contains('='));
        debug_assert!(!key.contains('\n'));

        self.0.push_str(key);

        self.0.push_str(match env_type {
            EnvType::Channel => "=ch:",
            EnvType::Devices => "=devices:",
            EnvType::VmSpace => "=vmspace:",
            EnvType::String => "=string:",
        });

        write!(&mut self.0, "{}\n", value).unwrap();
    }

    pub fn push_channel(&mut self, key: &str, ch: HandleId) {
        self.push(key, EnvType::Channel, ch.as_i32());
    }

    pub fn push_vmspace(&mut self, key: &str, vmspace: HandleId) {
        self.push(key, EnvType::VmSpace, vmspace.as_i32());
    }

    pub fn push_devices(&mut self, key: &str, devices: &[Device]) {
        let devices_json = serde_json::to_string(devices).unwrap();
        self.push(key, EnvType::Devices, devices_json);
    }

    pub fn push_string(&mut self, key: &str, value: &str) {
        self.push(key, EnvType::String, value);
    }
}

pub struct EnvironDeserializer<'a> {
    lines: Lines<'a>,
}

impl<'a> EnvironDeserializer<'a> {
    pub fn new(text: &'a str) -> EnvironDeserializer<'a> {
        EnvironDeserializer {
            lines: text.lines(),
        }
    }

    pub fn pop(&mut self) -> Option<(&'a str, EnvType, &'a str)> {
        let line = self.lines.next()?;
        let (key, value_with_prefix) = line.split_once('=').expect("malformed environ");
        let (prefix, value) = value_with_prefix
            .split_once(':')
            .expect("malformed environ");

        let env_type = match prefix {
            "ch" => EnvType::Channel,
            "devices" => EnvType::Devices,
            "vmspace" => EnvType::VmSpace,
            "string" => EnvType::String,
            _ => {
                panic!("invalid environ entry: {}", line);
            }
        };

        Some((key, env_type, value))
    }
}
