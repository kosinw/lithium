set confirm off
set architecture i386:x86-64
target remote 127.0.0.1:1234
symbol-file target/kernel
set disassemble-next-line auto
set disassembly-flavor intel