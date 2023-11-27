set confirm off
set architecture i386:x86-64
symbol-file target/obj/kernel.elf
set disassemble-next-line auto
set disassembly-flavor intel

target remote localhost:1234

br memory.rs:177
c