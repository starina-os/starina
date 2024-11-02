use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;

include!(concat!(env!("OUT_DIR"), "/autogen.rs"));
include!(concat!(env!("OUT_DIR"), "/startup_defs.rs"));

use starina_autogen::idl::startup::NewClient;
use starina_elf::Elf;
use starina_elf::PhdrType;
use starina_elf::ET_DYN;
use starina_elf::PF_R;
use starina_elf::PF_X;
use starina_types::address::PAddr;
use starina_types::address::VAddr;
use starina_types::environ::EnvironSerializer;
use starina_types::error::FtlError;
use starina_types::handle::HandleId;
use starina_types::handle::HandleRights;
use starina_types::idl::MovedHandle;
use starina_types::message::MessageBuffer;
use starina_types::message::MessageSerialize;
use starina_types::syscall::VsyscallPage;
use starina_types::vmspace::PageProtect;
use starina_utils::alignment::align_down;
use starina_utils::alignment::align_up;
use hashbrown::HashMap;

use crate::arch;
use crate::arch::paddr2vaddr;
use crate::arch::vaddr2paddr;
use crate::arch::PAGE_SIZE;
use crate::arch::USERSPACE_END;
use crate::arch::USERSPACE_START;
use crate::boot::BootInfo;
use crate::boot::USERMODE_ENABLED;
use crate::channel::Channel;
use crate::device_tree::DeviceTree;
use crate::folio::Folio;
use crate::handle::AnyHandle;
use crate::handle::Handle;
use crate::process::kernel_process;
use crate::process::Process;
use crate::process::KERNEL_VMSPACE;
use crate::refcount::SharedRef;
use crate::thread::Thread;
use crate::uaddr::UAddr;
use crate::vmspace::VmSpace;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    ParseElf(starina_elf::ParseError),
    AllocFolio(FtlError),
    NoPhdrs,
    NotPIE,
    NoRelaDyn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AppName(pub &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ServiceName(pub &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DepName(pub &'static str);

#[allow(unused)] // Can unused when no drivers are needed.
#[derive(Debug)]
pub enum WantedHandle {
    Service {
        dep_name: DepName,
        service_name: ServiceName,
    },
}

#[derive(Debug)]
pub enum WantedDevice {
    #[allow(unused)] // Can unused when no drivers are needed.
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
    device_tree: Option<&'a DeviceTree>,
    service_to_app_name: HashMap<ServiceName, AppName>,
    our_chs: HashMap<AppName, SharedRef<Channel>>,
    their_chs: HashMap<AppName, SharedRef<Channel>>,
    next_base_vaddr: VAddr,
}

impl<'a> StartupAppLoader<'a> {
    pub fn new(device_tree: Option<&DeviceTree>) -> StartupAppLoader {
        StartupAppLoader {
            device_tree,
            service_to_app_name: HashMap::new(),
            our_chs: HashMap::new(),
            their_chs: HashMap::new(),
            next_base_vaddr: USERSPACE_START,
        }
    }

    fn get_server_ch(&mut self, service_name: &ServiceName) -> AnyHandle {
        let (ch1, ch2) = Channel::new().unwrap();
        let handle_id = kernel_process()
            .handles()
            .lock()
            .add(Handle::new(ch1, HandleRights::ALL))
            .unwrap();

        let mut msgbuffer = MessageBuffer::new();
        (NewClient {
            handle: MovedHandle::new(handle_id).into(),
        })
        .serialize(&mut msgbuffer);

        let app_name = match self.service_to_app_name.get(service_name) {
            Some(app_name) => app_name,
            None => {
                panic!(
                    "service \"{}\" not found, available services are: {:?}",
                    service_name.0,
                    self.service_to_app_name.keys()
                );
            }
        };

        self.our_chs
            .get(app_name)
            .unwrap()
            .send(NewClient::MSGINFO, UAddr::from_kernel_ptr(&msgbuffer))
            .unwrap();

        Handle::new(ch2, HandleRights::ALL).into()
    }

    fn get_devices(&mut self, compat: &str) -> Vec<starina_types::environ::Device> {
        let mut devices = Vec::new();
        if let Some(device_tree) = self.device_tree.as_ref() {
            for device in device_tree.devices() {
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

                    devices.push(starina_types::environ::Device {
                        name: device.name.to_string(),
                        compatible: device.compatible.to_string(),
                        reg: device.reg,
                        interrupts,
                    });
                }
            }
        }

        if devices.is_empty() {
            warn!("no device found for {}", compat);
        }

        devices
    }

    fn create_process(
        &mut self,
        name: &AppName,
        entry_addr: usize,
        mut env: EnvironSerializer,
        vmspace: SharedRef<VmSpace>,
        handles: Vec<AnyHandle>,
        bootinfo: &BootInfo,
    ) {
        let proc = SharedRef::new(Process::create(vmspace.clone()));

        let mut handle_table = proc.handles().lock();
        for (i, handle) in handles.into_iter().enumerate() {
            let handle_id = handle_table.add(handle).unwrap();
            debug_assert_eq!(handle_id.as_i32(), (i + 1) as i32);
        }

        let startup_ch = self.their_chs.remove(name).unwrap();
        let startup_ch_handle = Handle::new(startup_ch, HandleRights::ALL);
        let startup_ch_id = handle_table.add(startup_ch_handle).unwrap();

        let vmspace_handle = Handle::new(vmspace.clone(), HandleRights::ALL);
        let vmspace_id = handle_table.add(vmspace_handle).unwrap();

        env.push_channel("dep:startup", startup_ch_id);
        env.push_vmspace("vmspace", vmspace_id);
        if let Some(cmdline) = &bootinfo.cmdline {
            env.push_string("boot_args", cmdline.as_str());
        }

        let env_str = env.finish();
        let environ_pages =
            Folio::alloc(align_up(env_str.len(), PAGE_SIZE)).expect("failed to allocate folio");
        let environ_pages_slice: &mut [u8] = unsafe {
            let vaddr = paddr2vaddr(environ_pages.paddr()).unwrap();
            core::slice::from_raw_parts_mut(vaddr.as_mut_ptr(), environ_pages.len())
        };
        environ_pages_slice[..env_str.len()].copy_from_slice(env_str.as_bytes());

        let vsyscall_page_paddr = {
            extern "C" {
                static __vsyscall_page: u8;
            }

            let page_addr = unsafe { &__vsyscall_page as *const u8 as usize };
            vaddr2paddr(VAddr::new(page_addr)).unwrap()
        };

        vmspace
            .map_vaddr_user(
                arch::VSYSCALL_ENTRY_ADDR,
                vsyscall_page_paddr,
                PAGE_SIZE,
                PageProtect::EXECUTABLE,
            )
            .unwrap();

        let environ_vaddr: VAddr = vmspace
            .map_anywhere_user(
                environ_pages.len(),
                Handle::new(SharedRef::new(environ_pages), HandleRights::ALL),
                PageProtect::READABLE,
            )
            .unwrap();

        let vsyscall_buffer = Folio::alloc(PAGE_SIZE).unwrap();
        let vsyscall_buffer_ptr = paddr2vaddr(vsyscall_buffer.paddr()).unwrap();
        unsafe {
            vsyscall_buffer_ptr
                .as_mut_ptr::<VsyscallPage>()
                .write(VsyscallPage {
                    entry: if USERMODE_ENABLED {
                        arch::VSYSCALL_ENTRY_ADDR.as_ptr()
                    } else {
                        arch::kernel_syscall_entry as *const _
                    },
                    environ_ptr: environ_vaddr.as_mut_ptr(),
                    environ_len: env_str.len(),
                });
        }

        let vsyscall_buffer_vaddr: VAddr = vmspace
            .map_anywhere_user(
                vsyscall_buffer.len(),
                Handle::new(SharedRef::new(vsyscall_buffer), HandleRights::ALL),
                PageProtect::READABLE,
            )
            .unwrap();

        let thread =
            Thread::spawn_kernel(proc.clone(), entry_addr, vsyscall_buffer_vaddr.as_usize());
        handle_table
            .add(Handle::new(thread, HandleRights::ALL))
            .unwrap();
    }

    fn load_app(&mut self, template: &AppTemplate, bootinfo: &BootInfo) -> Result<(), Error> {
        let base_vaddr = self.next_base_vaddr;
        let elf_loader = ElfLoader::parse(template.elf_file, base_vaddr)?;
        self.next_base_vaddr = self.next_base_vaddr.add(elf_loader.vmspace_len);
        if self.next_base_vaddr > USERSPACE_END {
            panic!("ran out of virtual address space");
        }

        trace!(
            "user app: name=\"{}\", base={}",
            template.name.0,
            base_vaddr
        );

        let vmspace = if USERMODE_ENABLED {
            KERNEL_VMSPACE.clone()
        } else {
            SharedRef::new(VmSpace::new().unwrap())
        };

        let entry_addr = elf_loader.load_into_memory(&vmspace)?;

        let mut env = EnvironSerializer::new();
        let mut handles = Vec::with_capacity(template.handles.len());
        for (i, wanted_handle) in template.handles.iter().enumerate() {
            let handle_id = HandleId::from_raw((i + 1).try_into().unwrap());
            let handle = match wanted_handle {
                WantedHandle::Service {
                    dep_name,
                    service_name,
                } => {
                    env.push_channel(&format!("dep:{}", dep_name.0), handle_id);
                    self.get_server_ch(service_name)
                }
            };

            handles.push(handle);
        }

        for wanted_device in template.devices {
            let WantedDevice::DeviceTreeCompatible(compat) = wanted_device;
            env.push_devices(compat, &self.get_devices(compat));
        }

        self.create_process(&template.name, entry_addr, env, vmspace, handles, bootinfo);
        Ok(())
    }

    pub fn load(&mut self, templates: &[AppTemplate], bootinfo: &BootInfo) {
        for t in templates {
            let (ch0, ch1) = Channel::new().unwrap();
            self.our_chs.insert(t.name, ch0);
            self.their_chs.insert(t.name, ch1);

            for service in t.provides {
                self.service_to_app_name.insert(*service, t.name);
            }
        }

        for t in templates {
            self.load_app(t, bootinfo).unwrap();
        }
    }
}

pub fn load_startup_apps(device_tree: Option<&DeviceTree>, bootinfo: &BootInfo) {
    StartupAppLoader::new(device_tree).load(STARTUP_APPS, bootinfo);
}

struct ElfLoader<'a> {
    elf_file: &'a [u8],
    elf: Elf<'a>,
    base_vaddr: VAddr,
    elf_paddr: PAddr,
    vmspace_len: usize,
}

impl<'a> ElfLoader<'a> {
    pub fn parse(elf_file: &[u8], base_vaddr: VAddr) -> Result<ElfLoader<'_>, Error> {
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

        let vmspace_len = align_up(highest_addr - lowest_addr, PAGE_SIZE);

        Ok(ElfLoader {
            elf_file,
            elf_paddr: vaddr2paddr(VAddr::new(elf_file.as_ptr() as usize)).unwrap(),
            elf,
            base_vaddr,
            vmspace_len,
        })
    }

    fn entry_addr(&self) -> usize {
        self.base_vaddr.as_usize() + (self.elf.ehdr.e_entry as usize)
    }

    fn map_segments(&mut self, vmspace: &SharedRef<VmSpace>) {
        for phdr in self.elf.phdrs {
            if phdr.p_type != starina_elf::PhdrType::Load {
                continue;
            }

            let mem_offset = phdr.p_vaddr as usize;
            let file_offset = phdr.p_offset as usize;
            let mem_size = phdr.p_memsz as usize;
            let file_size = phdr.p_filesz as usize;

            let mut offset = 0;
            while offset < mem_size {
                let file_part_len = core::cmp::min(file_size.saturating_sub(offset), PAGE_SIZE);
                let zero_part_len = PAGE_SIZE - file_part_len;

                let mut map_flags = PageProtect::zeroed();
                if phdr.p_flags & PF_R != 0 {
                    map_flags |= PageProtect::READABLE;
                }

                map_flags |= PageProtect::WRITABLE; // FIXME: needed for resolving relocs

                // if phdr.p_flags & PF_W != 0 {
                //     map_flags |= PageProtect::WRITABLE;
                // }

                if phdr.p_flags & PF_X != 0 {
                    map_flags |= PageProtect::EXECUTABLE;
                }

                let folio = if file_part_len > 0 {
                    let paddr_in_original = self.elf_paddr.add(file_offset + offset);
                    if !map_flags.contains(PageProtect::WRITABLE) {
                        // Read-only segment. No need to copy (assuming Rust
                        // guaranntees immutability of the segment).
                        Folio::alloc_shared(paddr_in_original, PAGE_SIZE).unwrap()
                    } else {
                        // Writable segment. We need to copy the segment to a
                        // new physical page so that multiple instances of the
                        // same app can have their own writable memory.
                        let folio = Folio::alloc(PAGE_SIZE).unwrap();
                        let folio_vaddr = paddr2vaddr(folio.paddr()).unwrap();
                        let copy_len = core::cmp::min(file_part_len, folio.len());
                        let dest_slice: &mut [u8] = unsafe {
                            core::slice::from_raw_parts_mut(folio_vaddr.as_mut_ptr(), copy_len)
                        };
                        dest_slice.copy_from_slice(
                            &self.elf_file
                                [(file_offset + offset)..(file_offset + offset + copy_len)],
                        );

                        folio
                    }
                } else {
                    Folio::alloc(PAGE_SIZE).unwrap()
                };

                let vaddr = VAddr::new(self.base_vaddr.as_usize() + mem_offset + offset);
                let aligned_vaddr = VAddr::new(align_down(vaddr.as_usize(), PAGE_SIZE));
                vmspace
                    .map_user(aligned_vaddr, folio, PAGE_SIZE, map_flags)
                    .unwrap();

                if zero_part_len > 0 {
                    // TODO: We might not need to zero-fill. Folio is already
                    //       zero-filled for security reasons.
                    let slice: &mut [u8] = unsafe {
                        core::slice::from_raw_parts_mut(
                            vaddr.add(file_part_len).as_mut_ptr(),
                            zero_part_len,
                        )
                    };

                    slice.fill(0);
                }

                offset += PAGE_SIZE;
            }
        }
    }

    fn get_shdr_by_name(&self, name: &str) -> Option<&starina_elf::Shdr> {
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

        use starina_elf::Rela;

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
                #[cfg(target_arch = "x86_64")]
                starina_elf::R_X86_64_RELATIVE => unsafe {
                    let base = self.base_vaddr.as_usize();
                    let ptr = (base + rela.r_offset as usize) as *mut i64;
                    *ptr += (base as i64) + rela.r_addend;
                },
                #[cfg(target_arch = "aarch64")]
                starina_elf::R_AARCH64_RELATIVE => unsafe {
                    let base = self.base_vaddr.as_usize();
                    let ptr = (base + rela.r_offset as usize) as *mut i64;
                    *ptr += (base as i64) + rela.r_addend;
                },
                #[cfg(target_arch = "riscv64")]
                starina_elf::R_RISCV_RELATIVE => unsafe {
                    let base = self.base_vaddr.as_usize();
                    let ptr = (base + rela.r_offset as usize) as *mut i64;
                    *ptr += (base as i64) + rela.r_addend;
                },
                _ => panic!("unsupported relocation type: {}", rela.r_info),
            }
        }

        Ok(())
    }

    pub fn load_into_memory(mut self, vmspace: &SharedRef<VmSpace>) -> Result<usize, Error> {
        vmspace.switch();
        self.map_segments(vmspace);
        self.relocate_rela_dyn()?;
        Ok(self.entry_addr())
    }
}
