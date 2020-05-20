use alloc::alloc::{GlobalAlloc, Layout};
use memory::frame_controller::FRAME_ALLOC;
use spin::Mutex;

struct HeapAllocator<'staticv> {}

enum HeapBlock {
    B_8,
    B_16,
    B_32,
    B_64,
    B_128,
    B_256,
    B_512,
    KB_P,
}

impl HeapBlock {
    pub fn best_match(size: u64) -> HeapBlockSize {}
}
