use core::arch::asm;

#[allow(non_camel_case_types)]
type c_long = i64;

pub enum Error {
    Unknown(#[allow(unused)] c_long),
}

/// See <https://github.com/riscv-non-isa/riscv-sbi-doc/blob/master/src/binary-encoding.adoc>
#[warn(clippy::too_many_arguments)]
unsafe fn sbi_call(
    a0: c_long,
    a1: c_long,
    a2: c_long,
    a3: c_long,
    a4: c_long,
    a5: c_long,
    fid: c_long,
    eid: c_long,
) -> Result<c_long, Error> {
    let error: c_long;
    let retval: c_long;

    unsafe {
        asm!(
            "ecall",
            inout("a0") a0 => error, inout("a1") a1 => retval, in("a2") a2,
            in("a3") a3, in("a4") a4, in("a5") a5, in("a6") fid, in("a7") eid,
        );
    }

    if error == 0 {
        Ok(retval)
    } else {
        Err(Error::Unknown(error))
    }
}

pub fn console_putchar(c: u8) {
    unsafe {
        let _ = sbi_call(c as c_long, 0, 0, 0, 0, 0, 0, 1);
    }
}

pub fn set_timer(time: u64) {
    unsafe {
        let _ = sbi_call(time as c_long, 0, 0, 0, 0, 0, 0, 0);
    }
}
