use core::arch::asm;
use core::arch::naked_asm;

use super::csr::StvecMode;
use super::csr::write_stvec;
use super::entry::trap_entry;
use super::interrupt::interrupt_handler;

/// The entry point of interrupts or exceptions.
#[naked]
#[repr(align(4))]
unsafe extern "C" fn idle_entry() -> ! {
    unsafe {
        naked_asm!(
            r#"
            j {resume_from_idle}
            "#,
            resume_from_idle = sym resume_from_idle,
        );
    }
}

fn resume_from_idle() -> ! {
    unsafe {
        write_stvec(trap_entry as *const () as usize, StvecMode::Direct);
    }

    interrupt_handler();
    todo!()
}

pub fn halt() -> ! {
    if cfg!(feature = "exit-on-idle") {
        // Use semihosting to shutdown the system.
        trace!("exiting with semihosting call...");

        #[repr(C, packed)]
        struct ExitParams {
            reason: u64,
            exit_code: u64,
        }

        let params = ExitParams {
            reason: 0x20026, // ADP_Stopped_ApplicationExit
            exit_code: 122,
        };

        unsafe {
            asm!(
                ".option push",

                ".option norvc", // Do not use compact instructions.
                ".balign 16",

                // Special sequence to trigger a semihosting syscall.
                "slli x0, x0, 0x1f",
                "ebreak",
                "srai x0, x0, 7",

                ".option pop",

                in("a0") 0x18, // SYS_EXIT
                in("a1") &raw const params as usize,
            );
        }
    }

    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

pub fn idle() -> ! {
    trace!("idle");

    if cfg!(feature = "exit-on-idle") {
        halt();
        panic!("failed to exit on idle");
    }

    unsafe {
        write_stvec(idle_entry as *const () as usize, StvecMode::Direct);

        // Memory fence to ensure writes so far become visible to other cores,
        // before entering WFI.
        asm!("fence");
        // Enable interrupts.
        asm!("csrsi sstatus, 1 << 1");
    }

    loop {
        unsafe {
            asm!("wfi");
        }
    }
}
