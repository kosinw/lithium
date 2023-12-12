set confirm off
set architecture i386:x86-64
set disassemble-next-line auto
set disassembly-flavor intel

gef-remote --qemu-user --qemu-binary target/obj/kernel.elf localhost 1234
file target/obj/kernel.elf