#![no_std]

use core::mem::size_of;

pub const R_RISCV_RELATIVE: u64 = 3;
pub const R_AARCH64_RELATIVE: u64 = 1027;

#[cfg(target_pointer_width = "64")]
pub type Addr = u64;
#[cfg(target_pointer_width = "64")]
pub type Off = u64;

#[cfg(target_pointer_width = "32")]
pub type Addr = u32;
#[cfg(target_pointer_width = "32")]
pub type Off = u32;

#[derive(Debug)]
#[repr(C)]
pub struct Ehdr {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: Addr,
    pub e_phoff: Off,
    pub e_shoff: Off,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

pub const ET_EXEC: u16 = 2;
pub const ET_DYN: u16 = 3;

#[repr(u32)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhdrType {
    Null = 0,
    Load = 1,
}

#[derive(Debug)]
#[repr(C)]
pub struct Phdr64 {
    pub p_type: PhdrType,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

#[cfg(target_pointer_width = "64")]
pub type Phdr = Phdr64;

impl Phdr {
    pub fn readable(&self) -> bool {
        self.p_flags & 0x4 != 0
    }

    pub fn writable(&self) -> bool {
        self.p_flags & 0x2 != 0
    }

    pub fn executable(&self) -> bool {
        self.p_flags & 0x1 != 0
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ShType {
    Null = 0,
    ProgBits = 1,
    SymTab = 2,
    StrTab = 3,
    Rela = 4,
}

#[derive(Debug)]
#[repr(C)]
pub struct Shdr64 {
    pub sh_name: u32,
    pub sh_type: u32,
    pub sh_flags: u64,
    pub sh_addr: Addr,
    pub sh_offset: Off,
    pub sh_size: u64,
    pub sh_link: u32,
    pub sh_info: u32,
    pub sh_addralign: u64,
    pub sh_entsize: u64,
}

#[cfg(target_pointer_width = "64")]
pub type Shdr = Shdr64;

#[derive(Debug)]
#[repr(C)]
pub struct Rela64 {
    pub r_offset: Addr,
    pub r_info: u64,
    pub r_addend: i64,
}

#[cfg(target_pointer_width = "64")]
pub type Rela = Rela64;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    BufferTooShort,
    InvalidMagic,
}

#[derive(Debug)]
pub struct Elf<'a> {
    pub ehdr: &'a Ehdr,
    pub phdrs: &'a [Phdr],
    pub shdrs: &'a [Shdr],
}

impl<'a> Elf<'a> {
    pub fn parse(buf: &[u8]) -> Result<Elf<'a>, ParseError> {
        if buf.len() < size_of::<Ehdr>() {
            return Err(ParseError::BufferTooShort);
        }

        let ehdr = unsafe { &*(buf.as_ptr() as *const Ehdr) };
        if ehdr.e_ident[0..4] != [0x7f, b'E', b'L', b'F'] {
            return Err(ParseError::InvalidMagic);
        }

        let phdrs_size = ehdr.e_phnum as usize * size_of::<Phdr>();
        if buf.len() < ehdr.e_phoff as usize + phdrs_size {
            return Err(ParseError::BufferTooShort);
        }

        let phdrs = unsafe {
            core::slice::from_raw_parts(
                buf.as_ptr().add(ehdr.e_phoff as usize) as *const Phdr,
                ehdr.e_phnum as usize,
            )
        };

        let shdrs = unsafe {
            core::slice::from_raw_parts(
                buf.as_ptr().add(ehdr.e_shoff as usize) as *const Shdr,
                ehdr.e_shnum as usize,
            )
        };

        Ok(Elf { ehdr, phdrs, shdrs })
    }
}
