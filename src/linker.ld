OUTPUT_FORMAT(elf64-x86-64)
ENTRY(start)

SECTIONS {
    . = 0x1000000;

    /* ensure that the bootloader entry code is at the beginning */
    .entry  : 
    {
        KEEP(*(.entry))
    }

    .text : {
        *(.text .text.*)
    }

    .rodata : {
        KEEP(*(.rodata .rodata.*))
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        *(.bss .bss.*)
    }
}
