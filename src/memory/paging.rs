use x86_64::{
    structures::paging::{frame::PhysFrame, page::Size4KiB, FrameAllocator, FrameDeallocator},
    PhysAddr,
};
use kernel::{PAGE_TABLE_END, PAGE_TABLE_START}

pub const NUMBER_OF_FRAMES: usize = (PAGE_TABLE_END - PAGE_TABLE_START) / Size4KiB.SIZE - 3;

pub struct page_allocator {
    avail_frame: [Option<PhysFrame<Size4KiB>>; NUMBER_OF_FRAMES],
    next: u32,
}

impl page_allocator {
    pub fn new() -> page_allocator {
        let mut frames = [None; NUMBER_OF_FRAMES];
        for (i, f) in (PAGE_TABLE_START..PAGE_TABLE_END)
            .step_by(Size4KiB.SIZE)
            .iter()
            .enumerate()
            .skip(3)
        {
            frames[i] = Some(PhysFrame::containing_address(PhysAddr::new(f)));
        }
        page_allocator { frames }
    }
}
