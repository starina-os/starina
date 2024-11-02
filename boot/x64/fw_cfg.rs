//! QEMU fw_cfg (Firmware Configuration) interface.
//!
//! <https://www.qemu.org/docs/master/specs/fw_cfg.html>
use core::arch::asm;

use starina_inlinedvec::InlinedString;

#[repr(u16)]
enum SelectorKey {
    Signature = 0x00,
    CmdlineSize = 0x14,
    CmdlineData = 0x15,
}

fn in8(port: u16) -> u8 {
    let data: u8;
    unsafe {
        asm!("in al, dx", in("dx") port, out("al") data, options(nostack));
    }
    data
}

fn out16(port: u16, data: u16) {
    unsafe {
        asm!("out dx, ax", in("dx") port, in("ax") data, options(nostack));
    }
}

fn write_selector_reg(key: SelectorKey) {
    out16(0x510, key as u16);
}

fn read_data_reg() -> u8 {
    in8(0x511)
}

pub struct FwCfg {
    pub cmdline: Option<InlinedString<126>>,
}

impl FwCfg {
    pub fn load() -> Option<FwCfg> {
        write_selector_reg(SelectorKey::Signature);
        let mut signature = [0u8; 4];
        for i in 0..4 {
            signature[i] = read_data_reg();
        }

        if &signature != b"QEMU" {
            return None;
        }

        write_selector_reg(SelectorKey::CmdlineSize);
        let mut cmdline_size = 0;
        for i in 0..4 {
            cmdline_size |= (read_data_reg() as usize) << (i * 8);
        }

        write_selector_reg(SelectorKey::CmdlineData);
        let mut cmdline_str: InlinedString<126> = InlinedString::new();
        for _ in 0..cmdline_size {
            let c = read_data_reg();
            if c == b'\0' {
                break;
            }

            cmdline_str.try_push_u8(c).unwrap();
        }

        Some(FwCfg {
            cmdline: Some(cmdline_str),
        })
    }
}
