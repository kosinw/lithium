# The Lithium kernel starts executing in this file, which is linked with
# the kernel Rust code so that it can refer to kernel symbols such as
# main().

.set STACKSIZE,		4096
.set PAGESIZE,		4096

.set CR0_PE,		1 << 0
.set CR0_WP,		1 << 16
.set CR0_PG,		1 << 31
.set CR0_BOOT64,	CR0_PE + CR0_WP + CR0_PG

.set CR4_PSE,		1 << 4
.set CR4_PAE,		1 << 5
.set CR4_FSGSBASE,	1 << 16
.set CR4_BOOT,		CR4_PAE + CR4_PSE + CR4_FSGSBASE

.set IA32_MSR_EFER,	0xc0000080
.set EFER_SCE,		1 << 0
.set EFER_LME,		1 << 8
.set EFER_NX,		1 << 11
.set EFER_BOOT_LO,	EFER_SCE + EFER_LME + EFER_NX
.set EFER_BOOT_HI,	0x00000000

.set GDT_NULL,		0
.set GDT_CODE64,	1 << 3
.set GDT32_CODE,	2 << 3
.set GDT32_DATA,	3 << 3

.set SEG_READ,		1 << 41
.set SEG_WRITE,		1 << 42
.set SEG_CODE,		1 << 43
.set SEG_DATA,		0 << 43
.set SEG_ALWAYS1,	1 << 44
.set SEG_PRESENT,	1 << 47
.set SEG_LONG,		1 << 53

.set SEG32_DEFAULT,	1 << 54
.set SEG32_GRANULARITY,	1 << 55
.set SEG32_LIMIT,	(0xF << 48) + 0xFFFF
.set SEG32_DEF,		SEG32_DEFAULT + SEG32_GRANULARITY + SEG32_LIMIT

.set KERNBASE,		0xFFFF800000000000

.set MULTIBOOT1_MAGIC,	0x1BADB002
.set MULTIBOOT1_FLAGS,	1 << 1	// Provide memory map.

.section .text.boot
.align 8
.globl multiboot1_header
multiboot1_header:
.int	MULTIBOOT1_MAGIC
.int	MULTIBOOT1_FLAGS
.int	-(MULTIBOOT1_MAGIC + MULTIBOOT1_FLAGS)

code32
.globl _start
_start:
    cli
    cld


