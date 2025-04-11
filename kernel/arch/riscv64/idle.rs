use core::arch::asm;
use core::arch::naked_asm;

use super::csr::StvecMode;
use super::csr::write_stvec;
use super::interrupt::interrupt_handler;
use super::transition::switch_to_kernel;

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
        write_stvec(switch_to_kernel as *const () as usize, StvecMode::Direct);
    }

    interrupt_handler();
    todo!()
}

pub fn halt() -> ! {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

pub fn idle() -> ! {
    trace!("idle");

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
