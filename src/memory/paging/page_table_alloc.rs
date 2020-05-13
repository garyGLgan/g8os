use x86_64::{
    structures::paging::{UnusedPhysFrame, PhysFrame, Size4KiB, FrameAllocator, FrameDeallocator},
    PhysAddr,
};
use crate::kernel_const::{PAGE_TABLE_END, PAGE_TABLE_START,};
use lazy_static::lazy_static;

const FRAME_SIZE: usize = 4096;
const NUMBER_OF_FRAMES: usize = ((PAGE_TABLE_END - PAGE_TABLE_START) / FRAME_SIZE as u64 - 3) as usize;

lazy_static!{
    static ref PAGE_TABLE_ALLOC: PageTableAlloc = {
        let mut frames = [0; NUMBER_OF_FRAMES];
        for (i, f) in (PAGE_TABLE_START..PAGE_TABLE_END)
            .step_by(FRAME_SIZE)
            .enumerate()
            .skip(3){
            frames[i] = f;
        }
        PageTableAlloc { avail_frame: frames, next: 0 }
    };
}

pub struct PageTableAlloc {
    avail_frame: [u64; NUMBER_OF_FRAMES],
    next: usize,
}

unsafe impl FrameAllocator<Size4KiB> for PageTableAlloc{
    fn allocate_frame(&mut self) -> Option<UnusedPhysFrame<Size4KiB>> {
        match self.next {
            i if i >= self.avail_frame.len() => None,
            j => unsafe {
                self.next +=1;
                Some(UnusedPhysFrame::new(PhysFrame::containing_address(PhysAddr::new(j as u64))))
            }
        }
    }
}
