use ftl_elf::Elf;
use ftl_elf::PhdrType;
use ftl_elf::ET_DYN;
use ftl_types::handle::HandleRights;
use ftl_types::syscall::VsyscallPage;
use ftl_utils::alignment::align_up;

use crate::arch::PAGE_SIZE;
use crate::handle::AnyHandle;
use crate::handle::Handle;
use crate::memory::AllocPagesError;
use crate::memory::AllocatedPages;
use crate::process::Process;
use crate::ref_counted::SharedRef;
use crate::thread::Thread;

#[derive(Debug)]
pub enum Error {
    ParseElf(ftl_elf::ParseError),
    NoPhdrs,
    AllocPages(AllocPagesError),
    NotPIE,
    #[cfg(target_arch = "riscv64")]
    NoRelaDyn,
}

pub struct KernelAppMemory {
    #[allow(dead_code)]
    pages: AllocatedPages,
}

pub struct AppLoader<'a> {
    elf_file: &'a [u8],
    elf: Elf<'a>,
    memory: AllocatedPages,
}

impl<'a> AppLoader<'a> {
    pub fn parse(elf_file: &[u8]) -> Result<AppLoader, Error> {
        let elf = Elf::parse(elf_file).map_err(Error::ParseElf)?;

        // TODO: Check DF_1_PIE flag to make sure it's a PIE, not a shared
        //       library.
        if elf.ehdr.e_type != ET_DYN {
            return Err(Error::NotPIE);
        }

        let lowest_addr = elf
            .phdrs
            .iter()
            .filter(|phdr| phdr.p_type == PhdrType::Load)
            .map(|phdr| phdr.p_vaddr as usize)
            .min()
            .ok_or(Error::NoPhdrs)?;
        let highest_addr = elf
            .phdrs
            .iter()
            .filter(|phdr| phdr.p_type == PhdrType::Load)
            .map(|phdr| (phdr.p_vaddr + phdr.p_memsz) as usize)
            .max()
            .ok_or(Error::NoPhdrs)?;

        let elf_len = align_up(highest_addr - lowest_addr, PAGE_SIZE);
        let memory = AllocatedPages::alloc(elf_len).map_err(Error::AllocPages)?;
        Ok(AppLoader {
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

    #[cfg(target_arch = "riscv64")]
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

    #[cfg(target_arch = "riscv64")]
    fn relocate_riscv(&mut self) -> Result<(), Error> {
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
            use ftl_elf::riscv::R_RISCV_RELATIVE;

            match rela.r_info {
                R_RISCV_RELATIVE => unsafe {
                    let ptr = (self.base_addr() + rela.r_offset as usize) as *mut i64;
                    *ptr += (self.base_addr() as i64) + rela.r_addend;
                },
                _ => panic!("unsupported relocation type: {}", rela.r_info),
            }
        }

        Ok(())
    }

    pub fn load(
        mut self,
        vsyscall_page: *const VsyscallPage,
        first_handle: AnyHandle,
    ) -> Result<SharedRef<Process>, Error> {
        self.load_segments();

        #[cfg(target_arch = "riscv64")]
        self.relocate_riscv()?;

        let entry = unsafe { core::mem::transmute(self.entry_addr()) };
        let proc = SharedRef::new(Process::create());
        let thread = Thread::spawn_kernel(proc.clone(), entry, vsyscall_page as usize);

        let kernel_app_memory = SharedRef::new(KernelAppMemory { pages: self.memory });

        {
            let mut handles = proc.handles().lock();
            handles.add(first_handle).unwrap();

            handles
                .add(Handle::new(kernel_app_memory, HandleRights::NONE))
                .unwrap();
            handles
                .add(Handle::new(thread, HandleRights::NONE))
                .unwrap();
        }

        Ok(proc)
    }
}
