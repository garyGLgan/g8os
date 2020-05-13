use crate::kernel_const::{BOOT_TMP_MMAP_BUFFER, FRAME_SIZE, MEMORY_RESERVED_BELOW};
use x86_64::PhysAddr;

struct MemoryMapItem {
    base_address: PhysAddr,
    size: u64,
    flags: u32,
    ext_flags: u32,
}

pub struct MemoryMapBuffer {
    len: u16,
    items: [MemoryMapItem; 1024], //assume the memroy map item less than 1024
}

struct UnusedBlock {
    size: u64,
    next: Option<&'static UnusedBlock>,
}

impl UnusedBlock {
    const fn new(size: u64) -> Self {
        UnusedBlock { size, next: None }
    }

    fn start_addr(&self) -> u64 {
        self as *const Self as u64
    }

    fn end_addr(&self) -> u64 {
        self.start_addr() + self.size * FRAME_SIZE
    }
}

pub struct PhysFrameAllocator {
    head: UnusedBlock,
}

impl PhysFrameAllocator {
    pub fn new(buf: &MemoryMapBuffer) -> Self {
        PhysFrameAllocator {
            head: UnusedBlock::new(0),
        }
    }
}
