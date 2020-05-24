// use alloc::alloc::{GlobalAlloc, Layout};
use crate::kernel_const::FRAME_SIZE;
use crate::memory::frame_controller::FRAME_ALLOC;
use crate::memory::paging::g8_page_table::PAGE_TABLE;
use spin::Mutex;
use x86_64::{
    structures::paging::{mapper::MapToError, PageTableFlags, Size2MiB, UnusedPhysFrame},
    VirtAddr,
};

const HEAP_MAX_BLOCKS: u64 = 0x4000000; // max heap size 128G
const HEAP_BLOCK_SIZE: u64 = 32;
const HEAP_MASK_START_ADDR: u64 = 0x20000000;
const HEAP_START_ADDR: u64 = 0x40000000;

struct FreeBlock<'a> {
    size: u64,
    prev: Option<&'a mut FreeBlock<'a>>,
    next: Option<&'a mut FreeBlock<'a>>,
}

struct HeapAllocator<'a> {
    mask: BitMask<'a>,
    head: FreeBlock<'a>,
    size: u64,
}

impl<'a> HeapAllocator<'a> {
    pub fn new() -> Self {
        let b = HEAP_MASK_START_ADDR as *mut [u64; HEAP_MAX_BLOCKS as usize];
        unsafe {
            Self {
                mask: BitMask {
                    size: 0,
                    inner: &mut *b,
                },
                head: FreeBlock {
                    size: 0,
                    prev: None,
                    next: None,
                },
                size: 0,
            }
        }
    }

    pub fn boundry_addr(&self) -> VirtAddr {
        VirtAddr::new(HEAP_START_ADDR + self.size)
    }

    pub fn expand(&mut self) -> Result<(), MapToError<Size2MiB>> {
        let alloc_frame =
            || -> Result<(UnusedPhysFrame<Size2MiB>, PageTableFlags), MapToError<Size2MiB>> {
                let frame = FRAME_ALLOC
                    .lock()
                    .allocate()
                    .ok_or(MapToError::FrameAllocationFailed)?;
                let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
                Ok((frame, flags))
            };

        let (frame, flags) = alloc_frame()?;
        self.expand_mask(frame, flags);

        let mut _size = 0;
        let s_addr = self.boundry_addr();
        for i in 0..(HEAP_BLOCK_SIZE * 8) {
            let (frame, flags) = alloc_frame()?;
            PAGE_TABLE
                .lock()
                .map_to(self.boundry_addr() + _size, frame, flags);
            _size += FRAME_SIZE;
        }
        self.size += _size;
        Ok(())
    }

    fn add_free_block(&mut self, addr: VirtAddr, size: u64) {
        let offset = (addr.as_u64() - HEAP_START_ADDR)>>5;
        if self.mask.is_set(offset-1){

        } else {

        }
        let mut cur = self.head;
        while let Some(mut ref block) = cur.next {

        }
    }

    fn expand_mask(&mut self, frame: UnusedPhysFrame<Size2MiB>, flags: PageTableFlags) {
        let s_ptr = self.mask.boudry_addr().as_u64() as *mut u64;
        PAGE_TABLE
            .lock()
            .map_to(self.mask.boudry_addr(), frame, flags);
        self.mask.size += 8 * FRAME_SIZE;
        let e_ptr = self.mask.boudry_addr().as_u64() as *mut u64;
        unsafe {
            s_ptr.write(0);
            e_ptr.write(0);
        }
    }
}

struct BitMask<'a> {
    size: u64,
    inner: &'a mut [u64; HEAP_MAX_BLOCKS as usize],
}

impl<'a> BitMask<'a> {
    fn boudry_addr(&self) -> VirtAddr {
        VirtAddr::new(
            (self.inner as *const [u64; HEAP_MAX_BLOCKS as usize] as u64) + (self.size >> 3),
        )
    }

    fn set_on(&mut self, pos: u64) {
        let (p, m) = self.split_pos(pos);
        self.inner[p] = self.inner[p] | m;
    }

    fn set_off(&mut self, pos: u64) {
        let (p, m) = self.split_pos(pos);
        self.inner[p] = self.inner[p] & !m;
    }

    fn split_pos(&self, pos: u64) -> (usize, u64) {
        assert!(pos < self.size);
        ((pos >> 6) as usize, 1 << (63 - (pos & 0x3f)))
    }

    fn is_set(&self, pos: u64) -> bool {
        let (p, m) = self.split_pos(pos);
        (self.inner[p] & m) != 0
    }
}
