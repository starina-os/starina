file target/kernel/debug/kernel
set confirm off
set history save on
set print pretty on
set disassemble-next-line auto
set architecture riscv:rv32
set riscv use-compressed-breakpoints yes
target remote 127.0.0.1:7778
