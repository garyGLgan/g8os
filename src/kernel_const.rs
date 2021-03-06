pub const PAGE_TABLE_START: u64 = 0x100000;
pub const PAGE_TABLE_P4: u64 = 0x100000;
pub const PAGE_TABLE_P3: u64 = 0x101000;
pub const PAGE_TABLE_P2: u64 = 0x102000;
pub const PAGE_TABLE_END: u64 = 0x400000;
pub const BOOT_TMP_MMAP_BUFFER: u64 = 0x2000;
pub const MEMORY_RESERVED_BELOW: u64 = 0x1000000;
pub const FRAME_SIZE: u64 = 0x200000;
pub const FRAME_SIZE_BIT_WIDTH: u64 = 21;

pub const STACK_BOTTOM: u64 = 0xc00000;
pub const STACK_TOP: u64 = 0xefffff;
