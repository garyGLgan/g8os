use x86_64::PhysAddr;
use x86_64::structures::paging::{
    PhysFrame,
    page_table::PageTableFlags, 
    UnusedPhysFrame, 
    Page, 
    MappedPageTable,
    PageTable, 
    Size2MiB,
    Size4KiB,
    Mapper,
    FrameAllocator,
    mapper::{PhysToVirt, MapperFlush, MapToError, UnmapError, FlagUpdateError, TranslateError}
};
use lazy_static::lazy_static;
use crate::kernel_const::{FRAME_SIZE, FRAME_SIZE_BIT_WIDTH, PAGE_TABLE_END, PAGE_TABLE_START};

pub const PAGE_FRAME_SIZE: usize = 4096;
pub const NUMBER_OF_FRAMES: usize =
    ((PAGE_TABLE_END - PAGE_TABLE_START) / PAGE_FRAME_SIZE as u64 - 3) as usize;

lazy_static!{
    pub static ref PAGE_TABLE: G8PagTable<'static> = {
        let l4_table = active_l4_page_table();
        let mut page_allocator:  PageTableAlloc = {
            let mut frames = [0; NUMBER_OF_FRAMES];
            for (i, f) in (PAGE_TABLE_START..PAGE_TABLE_END)
                .step_by(PAGE_FRAME_SIZE)
                .enumerate()
                .skip(3)
            {
                frames[i] = f;
            }
            PageTableAlloc {
                avail_frame: frames,
                next: 0,
            }
        };
        unsafe {
            G8PagTable::new(l4_table, page_allocator)
        }
    };
}

struct PageTableAlloc {
    avail_frame: [u64; NUMBER_OF_FRAMES],
    next: usize,
}

unsafe impl FrameAllocator<Size4KiB> for PageTableAlloc {
    fn allocate_frame(&mut self) -> Option<UnusedPhysFrame<Size4KiB>> {
        match self.next {
            i if i >= self.avail_frame.len() => None,
            j => unsafe {
                self.next += 1;
                Some(UnusedPhysFrame::new(PhysFrame::containing_address(
                    PhysAddr::new(j as u64),
                )))
            },
        }
    }
}


struct G8PageOffset {}

impl PhysToVirt for G8PageOffset {
    #[inline]
    fn phys_to_virt(&self, frame: PhysFrame) -> *mut PageTable {
        unsafe {
            &mut *(frame.start_address().as_u64() as *mut PageTable)
        }
    }
}

pub struct G8PagTable<'a> {
    inner: MappedPageTable<'a, G8PageOffset>, 
    allocator: PageTableAlloc,
}

fn active_l4_page_table() -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (frame, _) = Cr3::read();
    let addr = frame.start_address();
    unsafe {
        &mut *(addr.as_u64() as *mut PageTable)
    }
}


impl<'a> G8PagTable<'a>{
    unsafe fn new(l4_table: &'static mut PageTable, allocator: PageTableAlloc) -> Self {
        Self{
            inner: MappedPageTable::new(l4_table, G8PageOffset{}),
            allocator,
        }
    }

    fn map_to(
        &mut self,
        page: Page<Size2MiB>,
        frame: UnusedPhysFrame<Size2MiB>,
        flags: PageTableFlags,
    ) -> Result<MapperFlush<Size2MiB>, MapToError<Size2MiB>>{
        self.inner.map_to(page, frame, flags, &mut self.allocator)
    }

}

impl<'a> Mapper<Size2MiB> for G8PagTable<'a> {
    fn map_to<A>(
        &mut self,
        page: Page<Size2MiB>,
        frame: UnusedPhysFrame<Size2MiB>,
        flags: PageTableFlags,
        allocator: &mut A,
    ) -> Result<MapperFlush<Size2MiB>, MapToError<Size2MiB>>
    where  A: FrameAllocator<Size4KiB>{
        self.inner.map_to(page, frame, flags, allocator)
    }

    fn unmap(
        &mut self,
        page: Page<Size2MiB>,
    ) -> Result<(PhysFrame<Size2MiB>, MapperFlush<Size2MiB>), UnmapError> {
        self.inner.unmap(page)
    }

    fn update_flags(
        &mut self,
        page: Page<Size2MiB>,
        flags: PageTableFlags,
    ) -> Result<MapperFlush<Size2MiB>, FlagUpdateError> {
        self.inner.update_flags(page, flags)
    }

    fn translate_page(&self, page: Page<Size2MiB>) -> Result<PhysFrame<Size2MiB>, TranslateError> {
        self.inner.translate_page(page)
    }

}