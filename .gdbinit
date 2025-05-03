file build/kernel/debug/kernel
add-symbol-file apps/servers/lx/linux/vmlinux
set confirm off
set history save on
set print pretty on
set pagination off
set disassemble-next-line auto
set architecture riscv:rv64
set riscv use-compressed-breakpoints yes
target remote 127.0.0.1:7778
