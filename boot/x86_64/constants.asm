%define STAGE_0_LOADPOINT 0x7c00
%define STAGE_1_LOADPOINT 0x7e00
%define STAGE_2_LOADPOINT 0x8000
%define KERNEL_LOADPOINT 0x8200
%define SECTORS_PER_OPERATION 1
%define BOOT_DRIVER 0X7b00
%define DISK_LOAD_BUFFER 0xa000
%define BOOT_DISK_SECTOR_SIZE 512
%define DISK_LOAD_BUFFER_SIZE (SECTORS_PER_OPERATION * BOOT_DISK_SECTOR_SIZE)
%define GDT_ADDR 0x1000
%define GDT_SELECTOR_ZERO 0x0
%define GDT_SELECTOR_CODE 0x8
%define GDT_SELECTOR_DATA 0x10
%define IDT_ADDR 0x0
%define IDT_SIZE 0x1000
%define KERNEL_LOCATION 0x1000000
%define KERNEL_SIZE_LIMIT 0x200000
%define KERNEL_END 0x1200000
%define PAGE_SIZE_BYTES 2097152
%define PAGE_TABLES_LOCATION 0x10000000
%define PAGE_TABLES_SIZE_LIMIT 0x1000000
%define PAGE_TABLES_END 0x11000000
%define MEMORY_RESERVED_BELOW 0x11000000
%define DMA_MEMORY_START 0x20000
%define DMA_MEMORY_SIZE 0x70000
%define SYSCALL_STACK 0x11000000
%define BOOT_KERNEL_LOADPOINT 0x100000
%define BOOT_TMP_MMAP_BUFFER 0x2000
%define KERNEL_ENTRY_POINT 0x1000000
%define BOOTLOADER_SECTOR_COUNT 3
%define BOOT_PAGE_TABLE_SECTION_START 0x10000
%define BOOT_PAGE_TABLE_P4 0x10000
%define BOOT_PAGE_TABLE_P3 0x11000
%define BOOT_PAGE_TABLE_P2 0x12000
%define BOOT_PAGE_TABLE_SECTION_END 0x12000
%define PROCESS_COMMON_CODE 0x200000
%define PROCESS_STACK 0x400000
%define PROCESS_STACK_SIZE_PAGES 2
%define PROCESS_STACK_SIZE_BYTES 0x400000
%define PROCESS_STACK_END 0x800000
%define PROCESS_DYNAMIC_MEMORY 0x10000000000