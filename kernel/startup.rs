use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;

use ftl_autogen::protocols::autopilot::NewclientRequest;
use ftl_elf::Elf;
use ftl_elf::PhdrType;
use ftl_elf::ET_DYN;
use ftl_types::environ::EnvironSerializer;
use ftl_types::handle::HandleId;
use ftl_types::handle::HandleRights;
use ftl_types::message::MessageBuffer;
use ftl_types::message::MessageSerialize;
use ftl_types::message::MovedHandle;
use ftl_types::syscall::VsyscallPage;
use ftl_utils::alignment::align_up;
use hashbrown::HashMap;

use crate::arch::PAGE_SIZE;
use crate::channel::Channel;
use crate::device_tree::DeviceTree;
use crate::folio::Folio;
use crate::handle::AnyHandle;
use crate::handle::Handle;
use crate::memory::AllocPagesError;
use crate::memory::AllocatedPages;
use crate::process::kernel_process;
use crate::process::Process;
use crate::ref_counted::SharedRef;
use crate::syscall::syscall_entry;
use crate::thread::Thread;
use crate::vmspace::KERNEL_VMSPACE;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    ParseElf(ftl_elf::ParseError),
    NoPhdrs,
    AllocBuffer(AllocPagesError),
    NotPIE,
    NoRelaDyn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AppName(pub &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ServiceName(pub &'static str);

#[derive(Debug)]
pub enum WantedHandle {
    Service(ServiceName),
}

#[derive(Debug)]
pub enum WantedDevice {
    DeviceTreeCompatible(&'static str),
}

#[derive(Debug)]
pub struct AppTemplate {
    pub name: AppName,
    pub provides: &'static [ServiceName],
    pub elf_file: &'static [u8],
    pub handles: &'static [WantedHandle],
    pub devices: &'static [WantedDevice],
}

struct StartupAppLoader<'a> {
    device_tree: &'a DeviceTree,
    service_to_app_name: HashMap<ServiceName, AppName>,
    our_chs: HashMap<AppName, SharedRef<Channel>>,
    their_chs: HashMap<AppName, SharedRef<Channel>>,
}

impl<'a> StartupAppLoader<'a> {
    pub fn new(device_tree: &DeviceTree) -> StartupAppLoader {
        StartupAppLoader {
            device_tree,
            service_to_app_name: HashMap::new(),
            our_chs: HashMap::new(),
            their_chs: HashMap::new(),
        }
    }

    fn get_server_ch(&mut self, service_name: &ServiceName) -> AnyHandle {
        let (ch1, ch2) = Channel::new().unwrap();
        let handle_id = kernel_process()
            .handles()
            .lock()
            .add(Handle::new(ch1.into(), HandleRights::NONE))
            .unwrap();

        let mut msgbuffer = MessageBuffer::new();
        (NewclientRequest {
            handle: MovedHandle(handle_id),
        })
        .serialize(&mut msgbuffer);

        let app_name = self.service_to_app_name.get(service_name).unwrap();

        self.our_chs
            .get(app_name)
            .unwrap()
            .send(NewclientRequest::MSGINFO, &msgbuffer)
            .unwrap();

        Handle::new(ch2.into(), HandleRights::NONE).into()
    }

    fn get_devices(&mut self, compat: &str) -> Vec<ftl_types::environ::Device> {
        let mut devices = Vec::new();
        for device in self.device_tree.devices() {
            if device.compatible == compat {
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

                devices.push(ftl_types::environ::Device {
                    name: device.name.to_string(),
                    compatible: device.compatible.to_string(),
                    reg: device.reg,
                    interrupts,
                });
            }
        }

        if devices.is_empty() {
            panic!("no device found for {}", compat);
        }

        devices
    }

    fn create_process(
        &mut self,
        name: &AppName,
        entry_addr: usize,
        memory: AllocatedPages,
        mut env: EnvironSerializer,
        handles: Vec<AnyHandle>,
    ) {
        let entry = unsafe { core::mem::transmute(entry_addr) };
        let proc = SharedRef::new(Process::create());

        let mut handle_table = proc.handles().lock();
        let mut i = 0;
        for handle in handles {
            let handle_id = handle_table.add(handle).unwrap();
            debug_assert_eq!(handle_id.as_i32(), i + 1);
            i += 1;
        }

        let startup_ch = self.their_chs.remove(name).unwrap();
        let startup_ch_handle = Handle::new(startup_ch, HandleRights::NONE);
        let startup_ch_id = handle_table.add(startup_ch_handle).unwrap();

        let vmspace = KERNEL_VMSPACE.shared_ref();
        let vmspace_handle = Handle::new(vmspace, HandleRights::NONE);
        let vmspace_id = handle_table.add(vmspace_handle).unwrap();

        env.push_channel("dep:startup", startup_ch_id);
        env.push_vmspace("vmspace", vmspace_id);

        let env_str = env.finish();
        let mut environ_pages = AllocatedPages::alloc(align_up(env_str.len(), PAGE_SIZE))
            .expect("failed to allocate folio");
        environ_pages.as_slice_mut()[..env_str.len()].copy_from_slice(env_str.as_bytes());

        let vsyscall_buffer = AllocatedPages::alloc(PAGE_SIZE).unwrap();
        let vsyscall_ptr = vsyscall_buffer.as_ptr() as *mut VsyscallPage;
        unsafe {
            vsyscall_ptr.write(VsyscallPage {
                entry: syscall_entry,
                environ_ptr: environ_pages.as_ptr(),
                environ_len: env_str.len(),
            });
        }

        let thread = Thread::spawn_kernel(proc.clone(), entry, vsyscall_ptr as usize);

        // Copy into a folio.
        let mut environ_pages = AllocatedPages::alloc(align_up(env_str.len(), PAGE_SIZE))
            .expect("failed to allocate folio");
        environ_pages.as_slice_mut()[..env_str.len()].copy_from_slice(env_str.as_bytes());

        handle_table
            .add(Handle::new(thread, HandleRights::NONE))
            .unwrap();
        handle_table
            .add(Handle::new(
                SharedRef::new(Folio::from_allocated_pages(memory).unwrap()),
                HandleRights::NONE,
            ))
            .unwrap();
        handle_table
            .add(Handle::new(
                SharedRef::new(Folio::from_allocated_pages(vsyscall_buffer).unwrap()),
                HandleRights::NONE,
            ))
            .unwrap();
        handle_table
            .add(Handle::new(
                SharedRef::new(Folio::from_allocated_pages(environ_pages).unwrap()),
                HandleRights::NONE,
            ))
            .unwrap();
    }

    fn load_app(&mut self, template: &AppTemplate) -> Result<(), Error> {
        let elf_loader = ElfLoader::parse(template.elf_file)?;
        let (entry_addr, memory) = elf_loader.load_into_memory()?;

        let mut env = EnvironSerializer::new();
        let mut handles = Vec::with_capacity(template.handles.len());
        for (i, wanted_handle) in template.handles.iter().enumerate() {
            let handle_id = HandleId::from_raw((i + 1).try_into().unwrap());
            let handle = match wanted_handle {
                WantedHandle::Service(service_name) => {
                    env.push_channel(&format!("dep:{}", service_name.0), handle_id);
                    self.get_server_ch(&service_name)
                }
            };

            handles.push(handle);
        }

        for wanted_device in template.devices {
            let WantedDevice::DeviceTreeCompatible(compat) = wanted_device;
            env.push_devices(&compat, &self.get_devices(compat));
        }

        self.create_process(&template.name, entry_addr, memory, env, handles);
        Ok(())
    }

    pub fn load(&mut self, templates: &[AppTemplate]) {
        for t in templates {
            let (ch0, ch1) = Channel::new().unwrap();
            self.our_chs.insert(t.name, ch0);
            self.their_chs.insert(t.name, ch1);

            for service in t.provides {
                self.service_to_app_name.insert(*service, t.name);
            }
        }

        for t in templates {
            self.load_app(&t).unwrap();
        }
    }
}

pub fn load_startup_apps(templates: &[AppTemplate], device_tree: &DeviceTree) {
    StartupAppLoader::new(device_tree).load(templates);
}

struct ElfLoader<'a> {
    elf_file: &'a [u8],
    elf: Elf<'a>,
    memory: AllocatedPages,
}

impl<'a> ElfLoader<'a> {
    pub fn parse(elf_file: &'a [u8]) -> Result<ElfLoader, Error> {
        let elf = Elf::parse(elf_file).map_err(Error::ParseElf)?;

        // TODO: Check DF_1_PIE flag to make sure it's a PIE, not a shared
        //       library.
        if elf.ehdr.e_type != ET_DYN {
            return Err(Error::NotPIE);
        }

        let load_iter = elf
            .phdrs
            .iter()
            .filter(|phdr| phdr.p_type == PhdrType::Load);

        let lowest_addr = load_iter
            .clone()
            .map(|phdr| phdr.p_vaddr as usize)
            .min()
            .ok_or(Error::NoPhdrs)?;
        let highest_addr = load_iter
            .map(|phdr| (phdr.p_vaddr + phdr.p_memsz) as usize)
            .max()
            .ok_or(Error::NoPhdrs)?;

        let elf_len = align_up(highest_addr - lowest_addr, PAGE_SIZE);
        let memory = AllocatedPages::alloc(elf_len).map_err(Error::AllocBuffer)?;

        Ok(ElfLoader {
            elf_file,
            elf,
            memory,
        })
    }

    fn base_addr(&self) -> usize {
        self.memory.as_ptr() as usize
    }

    fn entry_addr(&self) -> usize {
        self.base_addr() + (self.elf.ehdr.e_entry as usize)
    }

    fn load_segments(&mut self) {
        let memory = self.memory.as_slice_mut();
        for phdr in self.elf.phdrs {
            if phdr.p_type != ftl_elf::PhdrType::Load {
                continue;
            }
            let mem_offset = phdr.p_vaddr as usize;
            let file_offset = phdr.p_offset as usize;
            let file_copy_len = phdr.p_filesz as usize;
            memory[mem_offset..mem_offset + file_copy_len]
                .copy_from_slice(&self.elf_file[file_offset..file_offset + file_copy_len]);
            let zeroed_len = phdr.p_memsz as usize - phdr.p_filesz as usize;
            memory[mem_offset + file_copy_len..mem_offset + file_copy_len + zeroed_len].fill(0);
        }
    }

    fn get_shdr_by_name(&self, name: &str) -> Option<&ftl_elf::Shdr> {
        fn get_cstr(buffer: &[u8], offset: usize) -> Option<&str> {
            let mut len = 0;
            while let Some(&ch) = buffer.get(offset + len) {
                if ch == 0 {
                    return core::str::from_utf8(&buffer[offset..offset + len]).ok();
                }
                len += 1;
            }
            None
        }

        let shstrtab_section = self.elf.shdrs.get(self.elf.ehdr.e_shstrndx as usize)?;
        let shstrtab = unsafe {
            core::slice::from_raw_parts(
                self.elf_file
                    .as_ptr()
                    .add(shstrtab_section.sh_offset as usize),
                shstrtab_section.sh_size as usize,
            )
        };

        self.elf.shdrs.iter().find(|shdr| {
            if let Some(sh_name) = get_cstr(shstrtab, shdr.sh_name as usize) {
                sh_name == name
            } else {
                false
            }
        })
    }

    fn relocate_rela_dyn(&mut self) -> Result<(), Error> {
        use core::mem::size_of;

        use ftl_elf::Rela;

        let rela_dyn = self.get_shdr_by_name(".rela.dyn").ok_or(Error::NoRelaDyn)?;
        let rela_entries = unsafe {
            assert!(
                rela_dyn.sh_size as usize % size_of::<Rela>() == 0,
                "misaligned .rela_dyn size"
            );

            core::slice::from_raw_parts(
                self.elf_file.as_ptr().add(rela_dyn.sh_offset as usize) as *const Rela,
                (rela_dyn.sh_size as usize) / size_of::<Rela>(),
            )
        };

        for rela in rela_entries {
            match rela.r_info {
                #[cfg(target_arch = "riscv64")]
                ftl_elf::R_RISCV_RELATIVE => unsafe {
                    let ptr = (self.base_addr() + rela.r_offset as usize) as *mut i64;
                    *ptr += (self.base_addr() as i64) + rela.r_addend;
                },
                #[cfg(target_arch = "aarch64")]
                ftl_elf::R_AARCH64_RELATIVE => unsafe {
                    let ptr = (self.base_addr() + rela.r_offset as usize) as *mut i64;
                    *ptr += (self.base_addr() as i64) + rela.r_addend;
                },
                _ => panic!("unsupported relocation type: {}", rela.r_info),
            }
        }

        Ok(())
    }

    pub fn load_into_memory(mut self) -> Result<(usize, AllocatedPages), Error> {
        self.load_segments();
        self.relocate_rela_dyn()?;
        Ok((self.entry_addr(), self.memory))
    }
}
