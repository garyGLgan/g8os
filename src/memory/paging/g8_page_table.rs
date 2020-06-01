use crate::kernel_const::{
    PAGE_TABLE_END, PAGE_TABLE_P2, PAGE_TABLE_P3, PAGE_TABLE_P4, PAGE_TABLE_START,
};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::structures::paging::{
    mapper::{FlagUpdateError, MapToError, MapperFlush, PhysToVirt, TranslateError, UnmapError},
    page_table::PageTableFlags,
    FrameAllocator, MappedPageTable, Mapper, Page, PageTable, PhysFrame, Size2MiB, Size4KiB,
    UnusedPhysFrame,
};
use x86_64::{PhysAddr, VirtAddr};

pub const PAGE_FRAME_SIZE: usize = 4096;
pub const NUMBER_OF_FRAMES: usize =
    ((PAGE_TABLE_END - PAGE_TABLE_START) / PAGE_FRAME_SIZE as u64 - 3) as usize;

lazy_static! {
    pub static ref PAGE_TABLE: Mutex<G8PagTable<'static>> = Mutex::new({
        let l4_table = active_l4_page_table();
        let page_allocator: PageTableAlloc = {
            let mut frames = [0; NUMBER_OF_FRAMES];
            let mut p = 0;
            for (_, f) in (PAGE_TABLE_START..(PAGE_TABLE_END - (PAGE_FRAME_SIZE as u64)))
                .step_by(PAGE_FRAME_SIZE)
                .enumerate()
            {
                match f {
                    PAGE_TABLE_P4 | PAGE_TABLE_P2 | PAGE_TABLE_P3 => {}
                    _ => {
                        frames[p] = f;
                        p += 1;
                    }
                }
            }
            PageTableAlloc {
                avail_frame: frames,
                next: 0,
            }
        };
        unsafe { G8PagTable::new(l4_table, page_allocator) }
    });
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
        unsafe { &mut *(frame.start_address().as_u64() as *mut PageTable) }
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
    unsafe { &mut *(addr.as_u64() as *mut PageTable) }
}

impl<'a> G8PagTable<'a> {
    unsafe fn new(l4_table: &'static mut PageTable, allocator: PageTableAlloc) -> Self {
        Self {
            inner: MappedPageTable::new(l4_table, G8PageOffset {}),
            allocator,
        }
    }

    pub fn map_to(
        &mut self,
        addr: VirtAddr,
        frame: UnusedPhysFrame<Size2MiB>,
        flags: PageTableFlags,
    ) -> Result<MapperFlush<Size2MiB>, MapToError<Size2MiB>> {
        let p = Page::<Size2MiB>::from_start_address(addr);

        match p {
            Ok(page) => self.inner.map_to(page, frame, flags, &mut self.allocator),
            _ => Err(MapToError::ParentEntryHugePage),
        }
    }

    pub fn unmap(
        &mut self,
        addr: VirtAddr,
    ) -> Result<(PhysFrame<Size2MiB>, MapperFlush<Size2MiB>), UnmapError> {
        let p = Page::<Size2MiB>::from_start_address(addr);
        match p {
            Ok(page) => self.inner.unmap(page),
            _ => Err(UnmapError::PageNotMapped),
        }
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
    where
        A: FrameAllocator<Size4KiB>,
    {
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

pub fn vaddr_from_table(l4: u32, l3: u32, l2: u32, offset: u32) -> VirtAddr {
    VirtAddr::new(((l4 as u64) << 39) | ((l3 as u64) << 30) | ((l2 as u64) << 21) | (offset as u64))
}
