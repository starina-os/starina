use core::arch::asm;
use core::arch::naked_asm;
use core::cell::RefMut;
use core::mem::offset_of;

use super::gdt::KERNEL_CS;
use super::idt::VECTOR_IRQ_BASE;
use super::interrupt;
use super::io_apic;
use super::local_apic;
use super::thread::Context;
use crate::refcount::SharedRef;
use crate::thread::Thread;

#[naked]
pub extern "C" fn kernel_syscall_entry(
    _a0: isize,
    _a1: isize,
    _a2: isize,
    _a3: isize,
    _a4: isize,
    _a5: isize,
) -> isize {
    unsafe {
        naked_asm!(
            r#"
                cli
                mov rax, gs:[{context_offset}]

                // Save general-purpose registers.
                mov [rax + {rbx_offset}], rbx
                mov [rax + {rcx_offset}], rcx
                mov [rax + {rdx_offset}], rdx
                mov [rax + {rsi_offset}], rsi
                mov [rax + {rdi_offset}], rdi
                mov [rax + {rbp_offset}], rbp
                mov [rax + {r8_offset}],  r8
                mov [rax + {r9_offset}],  r9
                mov [rax + {r10_offset}], r10
                mov [rax + {r11_offset}], r11
                mov [rax + {r12_offset}], r12
                mov [rax + {r13_offset}], r13
                mov [rax + {r14_offset}], r14
                mov [rax + {r15_offset}], r15

                mov rbx, rax
                pop rax
                mov [rbx + {rip_offset}], rax
                mov [rbx + {rsp_offset}], rsp

                // Handle the system call.
                call {syscall_handler}

                // Save the return value in the thread context, and switch
                // to the next thread.
                push rbx
                mov rbx, gs:[{context_offset}]
                mov [rbx + {rax_offset}], rax
                pop rbx

                jmp {switch_thread}
            "#,
            context_offset = const offset_of!(crate::arch::CpuVar, context),
            rip_offset = const offset_of!(Context, rip),
            rax_offset = const offset_of!(Context, rax),
            rbx_offset = const offset_of!(Context, rbx),
            rcx_offset = const offset_of!(Context, rcx),
            rdx_offset = const offset_of!(Context, rdx),
            rsi_offset = const offset_of!(Context, rsi),
            rdi_offset = const offset_of!(Context, rdi),
            rbp_offset = const offset_of!(Context, rbp),
            rsp_offset = const offset_of!(Context, rsp),
            r8_offset = const offset_of!(Context, r8),
            r9_offset = const offset_of!(Context, r9),
            r10_offset = const offset_of!(Context, r10),
            r11_offset = const offset_of!(Context, r11),
            r12_offset = const offset_of!(Context, r12),
            r13_offset = const offset_of!(Context, r13),
            r14_offset = const offset_of!(Context, r14),
            r15_offset = const offset_of!(Context, r15),
            syscall_handler = sym crate::syscall::syscall_handler,
            switch_thread = sym crate::thread::switch_thread,
        )
    }
}

pub fn return_to_user(thread: *mut super::Thread, sysret: Option<isize>) -> ! {
    let context: *mut Context = unsafe { &mut (*thread).context as *mut _ };
    if let Some(value) = sysret {
        unsafe {
            (*context).rax = value as usize;
        }
    }

    unsafe {
        asm!(
            r#"
                mov gs:[{context_offset}], rax

                // Restore general-purpose registers except RAX/RSP.
                mov rbx, [rax + {rbx_offset}]
                mov rcx, [rax + {rcx_offset}]
                mov rdx, [rax + {rdx_offset}]
                mov rsi, [rax + {rsi_offset}]
                mov rdi, [rax + {rdi_offset}]
                mov rbp, [rax + {rbp_offset}]
                mov r8,  [rax + {r8_offset}]
                mov r9,  [rax + {r9_offset}]
                mov r10, [rax + {r10_offset}]
                mov r11, [rax + {r11_offset}]
                mov r12, [rax + {r12_offset}]
                mov r13, [rax + {r13_offset}]
                mov r14, [rax + {r14_offset}]
                mov r15, [rax + {r15_offset}]

                // Build an IRET frame
                push 0                    // SS
                push [rax + {rsp_offset}] // RSP
                push 0x202                // RFLAGS (FIXME: should we save & restore RFLAGS?)
                push {iret_cs}            // CS
                push [rax + {rip_offset}] // RIP

                // Restore RAX
                mov rax, [rax + {rax_offset}]

                // Restore RIP/RSP and resume the thread execution.
                iretq
            "#,
            in ("rax") context as usize,
            iret_cs = const KERNEL_CS,
            context_offset = const offset_of!(crate::arch::CpuVar, context),
            rip_offset = const offset_of!(Context, rip),
            rax_offset = const offset_of!(Context, rax),
            rbx_offset = const offset_of!(Context, rbx),
            rcx_offset = const offset_of!(Context, rcx),
            rdx_offset = const offset_of!(Context, rdx),
            rsi_offset = const offset_of!(Context, rsi),
            rdi_offset = const offset_of!(Context, rdi),
            rbp_offset = const offset_of!(Context, rbp),
            rsp_offset = const offset_of!(Context, rsp),
            r8_offset = const offset_of!(Context, r8),
            r9_offset = const offset_of!(Context, r9),
            r10_offset = const offset_of!(Context, r10),
            r11_offset = const offset_of!(Context, r11),
            r12_offset = const offset_of!(Context, r12),
            r13_offset = const offset_of!(Context, r13),
            r14_offset = const offset_of!(Context, r14),
            r15_offset = const offset_of!(Context, r15),
            options(noreturn)
        );
    }
}

#[repr(C, packed)]
pub struct IrqFrame {
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rbp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    rdi: u64,
    error: u64,
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

#[no_mangle]
extern "C" fn x64_handle_interrupt(vector: u64, irq: *const IrqFrame) {
    match vector {
        0x00 => panic!("divide error: RIP={:x}", unsafe { (*irq).rip }),
        0x01 => panic!("debug exception"),
        0x02 => panic!("non-maskable interrupt"),
        0x03 => panic!("breakpoint"),
        0x04 => panic!("overflow"),
        0x05 => panic!("bound range exceeded"),
        0x06 => panic!("invalid opcode"),
        0x07 => panic!("device not available"),
        0x08 => panic!("double fault"),
        0x09 => panic!("coprocessor segment overrun"),
        0x0a => panic!("invalid TSS"),
        0x0b => panic!("segment not present"),
        0x0c => panic!("stack-segment fault"),
        0x0d => {
            panic!("general protection fault: RIP={:x}", unsafe { (*irq).rip },);
        }
        0x0e => {
            panic!(
                "page fault: RIP={:x}, CR2={:x}",
                unsafe { (*irq).rip },
                unsafe {
                    let cr2: u64;
                    asm!("mov rax, cr2", out("rax") cr2);
                    cr2
                }
            )
        }
        0x0f => panic!("reserved"),
        0x10 => panic!("x87 FPU error"),
        0x11 => panic!("alignment check"),
        0x12 => panic!("machine check"),
        0x13 => panic!("SIMD floating-point exception"),
        0x14 => panic!("virtualization exception"),
        0x15 => panic!("control protection exception"),
        _ if vector > VECTOR_IRQ_BASE as u64 => {
            let irq = vector - VECTOR_IRQ_BASE as u64;
            trace!("interrupt received: {}", irq);
            interrupt::handle_interrupt(irq as usize);
            local_apic::ack_interrupt();
        }
        _ => panic!("unexpected exception: {}", vector),
    }
}
