// Boot code ported from Kerla, which is authored by me.
// https://github.com/nuta/kerla/blob/bc2c48ccad1de79bbf488f82732037a8a1333ad3/runtime/x64/boot.S
.section ".text.boot", "ax"
.code32

// GDTR/GDT entries. This must be located in the .text.boot section as its
// address (gdt) must be physical.
.align 16
.global gdtr
gdtr:
    .word gdt_end - gdt - 1
    .quad gdt

.align 16
gdt:
    .quad 0x0000000000000000 // 0:  null descriptor
    .quad 0x00af9a000000ffff // 8:  64-bit code segment (kernel)
    .quad 0x00cf92000000ffff // 16: 64-bit data segment (kernel)
    .quad 0x00cf9a000000ffff // 24: 32-bit code segment (kernel)
gdt_end:

// The (old) multiboot header for QEMU -kernel.
// https://www.gnu.org/software/grub/manual/multiboot/multiboot.html
.align 8
multiboot_header:
.long 0x1badb002 // magic
.long 0x00000004 // flags: cmdline
.long -(0x1badb002 + 0x00000004)

// The entry point jumped from the bootloader.
.code32
.global boot
boot:
    cli
    cld

    // Set the boot (later reused for the cpu-local idle thread) stack.
    mov esp, offset __boot_stack

    // Save the multiboot magic and 64-bit physical address of multiboot info onto the stack.
    push 0      // Upper 32-bits.
    push eax
    push 0      // Upper 32-bits.
    push ebx


    // Prepare for RETF.
    mov eax, 24
    push eax
    lea edx, [protected_mode]
    push edx

    // Switch to our own temporary GDT.
    lgdt [gdtr]
    retf

protected_mode:
    mov ax, 16
    mov ds, ax
    mov ss, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

construct_page_table:
    // PML4: 0x00000000_00000000 (temporarily used in protected mode)
    lea edi, [__kernel_pml4]
    lea eax, [__kernel_pdpt + 0x103] // Present, writable, global.
    mov dword ptr [edi], eax
    mov dword ptr [edi + 4], 0

    // PML4: 0xffff8000_00000000
    lea edi, [__kernel_pml4 + 256 * 8]
    lea eax, [__kernel_pdpt + 0x103] // Present, writable, global.
    mov dword ptr [edi], eax
    mov dword ptr [edi + 4], 0

    // PDPT
    lea edi, [__kernel_pdpt]
    lea eax, [__kernel_pd + 0x103] // Present, writable, global.
    mov ecx, 4 // (# of PDPT entries)

write_pdpt_entry:
    mov dword ptr [edi], eax
    mov dword ptr [edi + 4], 0
    add eax, 0x1000
    add edi, 8
    loop write_pdpt_entry

    // Page Directory
    lea edi, [__kernel_pd]
    mov eax, 0x0000183 // Present, writable, global, page size is 2MB.
    mov ecx, 4 * 512 // (# of PDPT entries) * (# of entries in PD)

write_pd_entry:
    mov dword ptr [edi], eax
    mov dword ptr [edi + 4], 0
    add eax, 0x200000 // 2MB
    add edi, 8
    loop write_pd_entry

    jmp enable_long_mode

//
//  Common boot code for both BSP and APs.
//
enable_long_mode:
    // Enable PAE and PGE.
    mov eax, cr4
    or  eax, 0xa0
    mov cr4, eax

    // Set the page table address.
    lea eax, [__kernel_pml4]
    mov cr3, eax

    // Enable long mode.
    mov ecx, 0xc0000080
    rdmsr
    or  eax, 0x0100
    wrmsr

    // Prepare for RETF.
    mov  eax, 8
    push eax
    lea  edx, [long_mode_in_low_address]
    push edx

    // Enable paging.
    mov eax, cr0
    or  eax, 0x80000000
    mov cr0, eax

    retf

.code64
long_mode_in_low_address:
    mov ax, 0
    mov ds, ax
    mov ss, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    // Update RSP/RIP to use the virtual address.
    mov rbx, 0xffff800000000000
    or  rsp, rbx
    lea rax, [long_mode - 0xffff800000000000]
    or  rax, rbx
    jmp rax

//
//  From here, we're in the .text section: we no longer use physical address.
//
.code64
.text
long_mode:
    // Determine whether the current CPU is BSP or AP.
    mov esi, 0xfee00020
    mov eax, [esi]
    shr eax, 24
    test eax, eax
    jz  setup_bsp

setup_ap:
    // TODO: SMP
    ud2

setup_bsp:
    // Clear .bss section
    mov al, 0x00
    lea rdi, [rip + __bss]
    lea rcx, [rip + __bss_end]
    sub rcx, rdi
    cld
    rep stosb

    pop  rsi // the address of multiboot info
    pop  rdi // multiboot magic

    // Clear the frame pointer to stop backtracing here.
    xor rbp, rbp

    lea  rax, [rip + x64_boot]
    call rax

// In case init() returns.
halt:
    cli
    hlt
    jmp halt
