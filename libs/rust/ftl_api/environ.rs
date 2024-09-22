//! Environ, a collection of key-value pairs passed to the application.
use alloc::vec::Vec;
use core::fmt;

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

/// Environ, short for *environment*, is a collection of key-value pairs that
/// are used to:
///
/// - Dependency injection. Especially channels connected to dependent services.
/// - Configuration settings.
/// - Command-line arguments (shell is not available as of this writing though!).
/// - The [`VmSpace`] of the current process. To manage its own address space.
///
/// # Environ is a key-value store
///
/// The keys are always strings, and the values can be of different types.
/// Currently, the supported types are:
///
/// - Channel.
/// - VmSpace.
/// - A list of found devices (for device drivers).
///
/// # How to request environ items
///
/// To request an environ item,
///
/// # Examples
///
/// ```
/// pub fn main(mut env: Environ) {
///     // Dump all environ items.
///     info!("env: {:#?}", env);
///
///     // Take the ownership of the channel.
///     let driver_ch: Channel = env.take_channel("dep:ethernet_device").unwrap();
/// }
/// ```
///
/// This snippet logs:
///
/// ```text
/// [tcpip       ] INFO   env: {
///     "dep:ethernet_device": Channel(
///         Channel(#1),
///     ),
///     "dep:startup": Channel(
///         Channel(#2),
///     ),
/// }
/// ```
///
/// # Difference from environment variables
///
/// Environ is similar to environment variables in POSIX, and actually, internal
/// implementation is mostly the same (both key and value are strings). However,
/// the key difference is that FTL enforces convention on key names so that we can
/// provide a consistent and type-safe API.
///
/// Otherwise, each application would have different command-line parsers.
pub struct Environ {
    entries: HashMap<&'static str, Value>,
}

impl Environ {
    pub(crate) fn parse(raw: &'static str) -> Environ {
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

    /// Returns the channel associated with the key.
    ///
    /// If the key is not found, or is already taken, `None` is returned.
    ///
    /// # Panics
    ///
    /// Panics if the value associated with the key is not a channel.
    pub fn take_channel(&mut self, key: &str) -> Option<Channel> {
        match self.entries.remove(key) {
            Some(Value::Channel(channel)) => Some(channel),
            Some(_) => panic!("not a channel"),
            None => None,
        }
    }

    /// Returns the vmspace associated with the key.
    ///
    /// If the key is not found, or is already taken, `None` is returned.
    ///
    /// # Panics
    ///
    /// Panics if the value associated with the key is not a vmspace.
    pub fn take_vmspace(&mut self, key: &str) -> Option<VmSpace> {
        match self.entries.remove(key) {
            Some(Value::VmSpace(vmspace)) => Some(vmspace),
            Some(_) => panic!("not a channel"),
            None => None,
        }
    }

    /// Returns the devices associated with the key.
    pub fn devices(&self, key: &str) -> Option<&[Device]> {
        match self.entries.get(key) {
            Some(Value::Devices(devices)) => Some(devices),
            Some(_) => panic!("not devices"),
            None => None,
        }
    }
}

impl fmt::Debug for Environ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.entries.iter().map(|(k, v)| (k, v)))
            .finish()
    }
}
