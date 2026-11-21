/*
 * Integrate OS — x86_64 Bare-Metal Kernel Linker Script (Phase 1)
 *
 * Physical load address: 1 MiB (0x00100000), above legacy BIOS/VGA regions.
 * The .multiboot section is placed at the start of the first load segment
 * so GRUB / Multiboot loaders locate the magic header within the first 8 KiB.
 */

ENTRY(_start)
OUTPUT_FORMAT(elf64-x86-64)

KERNEL_PHYS_BASE = 0x00100000;

SECTIONS
{
    . = KERNEL_PHYS_BASE;
    __kernel_start = .;

    .multiboot ALIGN(8) : AT(ADDR(.multiboot))
    {
        KEEP(*(.multiboot))
    }

    .text ALIGN(4K) : AT(ADDR(.text))
    {
        __text_start = .;
        KEEP(*(.text.boot))
        *(.text .text.*)
        __text_end = .;
    }

    .rodata ALIGN(4K) : AT(ADDR(.rodata))
    {
        __rodata_start = .;
        *(.rodata .rodata.*)
        __rodata_end = .;
    }

    .data ALIGN(4K) : AT(ADDR(.data))
    {
        __data_start = .;
        *(.data .data.*)
        *(.got .got.*)
        __data_end = .;
    }

    .bss ALIGN(4K) : AT(ADDR(.bss))
    {
        __bss_start = .;
        KEEP(*(.bss.page_tables))
        . = ALIGN(16);
        __stack_bottom = .;
        KEEP(*(.bss.stack))
        . = ALIGN(16);
        __stack_top = .;
        *(COMMON)
        *(.bss .bss.*)
        __bss_end = .;
    }

    . = ALIGN(4K);
    __kernel_end = .;

    /DISCARD/ :
    {
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.note .note.*)
        *(.comment)
    }
}
