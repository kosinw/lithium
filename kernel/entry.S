;;
;;  entry.S - Multiboot header and kernel entry point.
;;

global  _start
global  _start64
global  bootpgtbl

extern  kernel_main

KERNBASE        equ 0xffff800000000000
PAGESIZE        equ 4096

MAGIC           equ 0x1BADB002
MBALIGN         equ 1 << 0
MBINFO          equ 1 << 1
MBFLAGS         equ MBALIGN | MBINFO
CHECKSUM        equ -(MAGIC + MBFLAGS)

PAGE_PRESENT    equ 1 << 0
PAGE_WRITABLE   equ 1 << 1
PAGE_HUGE       equ 1 << 7
PAGE_NX         equ 1 << 63

CR0_PE          equ 1 << 0
CR0_WP          equ 1 << 16
CR0_PG          equ 1 << 31
CR0_BOOT        equ CR0_PE | CR0_WP | CR0_PG

CR4_PSE         equ 1 << 4
CR4_PAE         equ 1 << 5
CR4_FSGSBASE    equ 1 << 16
CR4_BOOT        equ CR4_PSE | CR4_PAE | CR4_FSGSBASE

EFER_SCE        equ 1 << 0
EFER_LME        equ 1 << 8
EFER_NX         equ 1 << 11
EFER_BOOT       equ EFER_SCE | EFER_LME | EFER_NX

SEG_CODE        equ 1 << 43
SEG_READ        equ 1 << 41
SEG_WRITE       equ 1 << 42
SEG_PRESENT     equ 1 << 47
SEG_ALWAYS1     equ 1 << 44
SEG_LONG        equ 1 << 53

section .text.boot
align 4
    dd      MAGIC
    dd      MBFLAGS
    dd      CHECKSUM

bits 32
_start:
    cli

    ; Setup initial stack.
    mov     esp, stack0 - KERNBASE + PAGESIZE

    ; Setup identity paging before entering long mode.
    ; We identity map the first 512 2MiB pages into the page table.
    mov     eax, bootpgtbl - KERNBASE
    mov     cr3, eax

    ; Enable PAE, PSE, and FSGSBASE flags in CR4 register.
    mov     eax, cr4
    or      eax, CR4_BOOT
    mov     cr4, eax

    ; Enable long mode, NX bit, and syscall extensions in EFER register.
    mov     ecx, 0xc0000080
    rdmsr
    or      eax, EFER_BOOT
    wrmsr

    ; Enable paging in the CR0 register.
    mov     eax, cr0
    or      eax, CR0_BOOT
    mov     cr0, eax

    ; Load GDT descriptor for long mode.
    lgdt    [bootgdt.desc - KERNBASE]

    ; Jump into long mode code!
    jmp     0x8:(_start64 - KERNBASE)

    ret

section .text
bits 64
_start64:
    ; Clear segment registers.
    xor     ax, ax
    mov     ds, ax
    mov     ss, ax
    mov     es, ax
    mov     fs, ax
    mov     gs, ax

    ; Setup the 64-bit stack register.
    mov     rsp, stack0 + PAGESIZE

    ; Return to label to properly setup GDB stack info.
    mov     rax, qword .1
    push    qword rax
    ret

.1:
    ; Set up kernel_main arguments to have multiboot response structure.
    mov     rdi, rbx

    ; Set up base pointer register.
    push    qword 0
    mov     rbp, rsp

    ; Call kernel_main in Rust code.
    call    kernel_main

.2:
    cli
    hlt
    jmp     .2

section .rodata
align PAGESIZE
bootpgtbl:
bootpgtbl4:
    dq      (bootpgtbl3 - KERNBASE) + (PAGE_WRITABLE | PAGE_PRESENT)
    resb    (PAGESIZE>>1) - 8
    dq      (bootpgtbl3 - KERNBASE) + (PAGE_WRITABLE | PAGE_PRESENT)
    resb    (PAGESIZE>>1) - 8

align PAGESIZE
bootpgtbl3:
    dq      (bootpgtbl2 - KERNBASE) + (PAGE_WRITABLE | PAGE_PRESENT)
    resb    PAGESIZE - 8

align PAGESIZE
bootpgtbl2:
    dq      (0 << 20) + (PAGE_HUGE | PAGE_WRITABLE | PAGE_PRESENT)
%assign i 1
%rep 511
    dq      ((2*i) << 20) + (PAGE_NX | PAGE_HUGE | PAGE_WRITABLE | PAGE_PRESENT)
%assign i i+1
%endrep

align 16
bootgdt:
    dq      0   ; zero entry
    dq      SEG_ALWAYS1 | SEG_CODE | SEG_READ | SEG_LONG | SEG_PRESENT    ; long mode code segment
.desc:
    dw      ($)-bootgdt-1
    dq      (bootgdt-KERNBASE)

section .bss
align PAGESIZE
stack0: resb PAGESIZE