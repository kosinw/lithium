/* We want the symbol _start to be our entry point */
ENTRY(_start)

SECTIONS
{
    . = 0xffff800000000000;
    PROVIDE(KERNBASE = .);

    . = 0xffff800000100000;
    PROVIDE(KERNSTART = .);
    .text BLOCK(4096) : ALIGN(4096)
    {
        KEEP(*(.text.boot))
        *(.text .text.*)
        . = ALIGN(4096);
        *(.trap)
    }
    PROVIDE(TEXTEND = .);

    .rodata  BLOCK(4096) : ALIGN(4096)
    {
        *(.rodata .rodata.*)
    }

    .data BLOCK(4096) : ALIGN(4096)
    {
        *(.data .data.*)
    }

    .bss BLOCK(4096) : ALIGN(4096)
    {
        *(.bss .bss.*)
        *(COMMON)
    }

    . = ALIGN(4096);

    PROVIDE(KERNSTOP = .);

    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
    }
}
