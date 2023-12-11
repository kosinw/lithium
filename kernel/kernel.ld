/* We want the symbol _start to be our entry point */
ENTRY(_start)

SECTIONS
{
    . = 0x100000;

    PROVIDE(__kernel_start = .);
    .text BLOCK(4096) : ALIGN(4096)
    {
        KEEP(*(.text.boot))
        *(.text .text.*)
        . = ALIGN(4096);
        *(.trap)
        . = ALIGN(4096);
    }

    .rodata  BLOCK(4096) : ALIGN(4096)
    {
        *(.rodata .rodata.*)
        . = ALIGN(4096);
    }

    PROVIDE(__data_start = .);
    .data BLOCK(4096) : ALIGN(4096)
    {
        *(.data .data.*)
        . = ALIGN(4096);
    }

    .bss BLOCK(4096) : ALIGN(4096)
    {
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4096);
    }

    /DISCARD/ BLOCK(0x100000) : ALIGN(0x100000) {
        *(.eh_frame)
        *(.note .note.*)
    }

    . = ALIGN(0x100000);

    PROVIDE(__kernel_end = .);
}
