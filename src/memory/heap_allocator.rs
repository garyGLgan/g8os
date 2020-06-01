use crate::kernel_const::FRAME_SIZE;
use crate::memory::frame_controller::FRAME_ALLOC;
use crate::memory::paging::g8_page_table::PAGE_TABLE;
use crate::util::Locked;
use alloc::alloc::{GlobalAlloc, Layout};
use x86_64::{
    structures::paging::{mapper::MapToError, PageTableFlags, Size2MiB, UnusedPhysFrame},
    VirtAddr,
};

const HEAP_MAX_BLOCKS: u64 = 0x4000000; // max heap size 128G
const HEAP_BLOCK_SIZE: u64 = 64; // matches cache line
const HEAP_BLOCK_SIZE_BW: u64 = 6; // bit width of heap block size
const HEAP_MASK_START_ADDR: u64 = 0x20000000;
const HEAP_START_ADDR: u64 = 0x40000000;

#[global_allocator]
static ALLOCATOR: Locked<HeapAllocator> = Locked::new(HeapAllocator::new());

struct BitMask {
    size: u64,
    inner: Option<&'static mut [u64; HEAP_MAX_BLOCKS as usize]>,
}

impl BitMask {
    fn boudry_addr(&self) -> u64 {
        HEAP_MASK_START_ADDR + (self.size >> 3)
    }

    unsafe fn inner(&mut self) -> &mut [u64; HEAP_MAX_BLOCKS as usize] {
        self.inner.as_mut().unwrap()
    }

    unsafe fn set_on(&mut self, pos: u64) {
        let (p, m) = self.split_pos(pos);
        let inner = self.inner();
        inner[p] = inner[p] | m;
    }

    unsafe fn set_off(&mut self, pos: u64) {
        let (p, m) = self.split_pos(pos);
        let inner = self.inner();
        inner[p] = inner[p] & !m;
    }

    fn split_pos(&self, pos: u64) -> (usize, u64) {
        assert!(pos < self.size);
        ((pos >> 6) as usize, 1 << (63 - (pos & 0x3f)))
    }

    unsafe fn is_set(&mut self, pos: u64) -> bool {
        let (p, m) = self.split_pos(pos);
        let inner = self.inner();
        inner[p] & m != 0
    }

    fn is_empty(&self) -> bool {
        self.size == 0
    }
}

struct FreeBlock {
    size: u64,
    prev: Option<&'static mut FreeBlock>,
    next: Option<&'static mut FreeBlock>,
}

impl FreeBlock {
    fn start_addr(&self) -> u64 {
        self as *const Self as u64
    }

    fn end_addr(&self) -> u64 {
        self.start_addr() + self.size
    }

    unsafe fn expand_backward(&mut self, addr: u64, size: u64) -> &mut Self {
        assert!(addr == self.end_addr());
        self.size += size;
        self.write_addr_at_end();
        self
    }

    unsafe fn expand_forward(&mut self, addr: u64, size: u64) -> &mut Self {
        assert!(addr + size == self.start_addr());

        let block = FreeBlock {
            size: self.size + size,
            prev: self.prev.take(),
            next: self.next.take(),
        };
        let _ptr = addr as *mut FreeBlock;
        _ptr.write(block);
        (&mut *_ptr).write_addr_at_end();
        if let Some(ref mut prev) = self.prev {
            prev.next = Some(&mut *_ptr);
        }
        if let Some(ref mut next) = self.next {
            next.prev = Some(&mut *_ptr);
        }
        &mut *_ptr
    }

    unsafe fn merge_next(&mut self) -> &mut Self {
        let _s_addr = self.start_addr();
        let _e_addr = self.end_addr();
        
        if let Some(ref mut next) = self.next {
            if _e_addr == next.start_addr() {
                let block = &mut *(_e_addr as *mut FreeBlock);
                match block.next {
                    None => self.next = None,
                    Some(ref mut b) => {
                        let _addr = b.start_addr();
                        self.next = Some(&mut *(_addr as *mut FreeBlock));
                        b.prev = Some(&mut *(self.start_addr() as *mut FreeBlock));
                    }
                }
                self.expand_backward(block.start_addr(), block.size);
                self.write_addr_at_end();
            }
        }
        self
    }

    unsafe fn alloc(&mut self, size: u64) -> *mut u8 {
        assert!(self.size >= size);
        self.size -= size;
        self.write_addr_at_end();
        self.end_addr() as *mut u8
    }

    unsafe fn release(&mut self) -> *mut u8 {
        if let Some(ref mut prev) = self.prev {
            prev.next = self.next.take();
        }
        if let Some(ref mut next) = self.next {
            next.prev = self.prev.take();
        }
        self.start_addr() as *mut u8
    }

    unsafe fn write_addr_at_end(&self) {
        let _e_ptr = (self.end_addr() - 8) as *mut u64;
        _e_ptr.write(self.start_addr());
    }
}

struct HeapAllocator {
    mask: BitMask,
    head: FreeBlock,
    size: u64,
}

impl HeapAllocator {
    const fn new() -> Self {
        Self {
            mask: BitMask {
                size: 0,
                inner: None,
            },
            head: FreeBlock {
                size: 0,
                prev: None,
                next: None,
            },
            size: 0,
        }
    }

    pub fn boundry_addr(&self) -> VirtAddr {
        VirtAddr::new(HEAP_START_ADDR + self.size)
    }

    pub unsafe fn expand(&mut self) -> Result<(), MapToError<Size2MiB>> {
        let alloc_frame =
            || -> Result<(UnusedPhysFrame<Size2MiB>, PageTableFlags), MapToError<Size2MiB>> {
                let frame = FRAME_ALLOC
                    .lock()
                    .allocate()
                    .ok_or(MapToError::FrameAllocationFailed)?;
                let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
                Ok((frame, flags))
            };

        let mut _size = 0;
        let s_addr = self.boundry_addr();
        for _ in 0..(HEAP_BLOCK_SIZE * 8) {
            let (frame, flags) = alloc_frame()?;
            PAGE_TABLE
                .lock()
                .map_to(self.boundry_addr() + _size, frame, flags);
            _size += FRAME_SIZE;
        }
        self.size += _size;
        let (frame, flags) = alloc_frame()?;
        self.expand_mask(frame, flags);

        self.ins_merg_free_block(s_addr.as_u64(), _size);
        Ok(())
    }

    unsafe fn ins_merg_free_block(&mut self, addr: u64, size: u64) {
        let s_off = (addr - HEAP_START_ADDR) >> HEAP_BLOCK_SIZE_BW;
        let e_off = (addr + size - HEAP_START_ADDR) >> HEAP_BLOCK_SIZE_BW;
        let node = if s_off > 0 && !self.mask.is_set(s_off - 1) {
            let _addr = ((addr - 8) as *const u64).read();
            let _block = &mut *(_addr as *mut FreeBlock);
            _block.expand_backward(addr, size);
            if e_off < self.mask.size && !self.mask.is_set(e_off) {
                _block.merge_next();
            }
            _block
        } else if e_off < self.mask.size && !self.mask.is_set(e_off) {
            let _block = &mut *((addr + size) as *mut FreeBlock);
            _block.expand_forward(addr, size)
        } else {
            self.add_free_block(addr, size);
            self.head.next.as_mut().unwrap()
        };
        self.mask
            .set_off((node.start_addr() - HEAP_START_ADDR) >> HEAP_BLOCK_SIZE_BW);
        self.mask
            .set_off((node.end_addr() - HEAP_START_ADDR - 1) >> HEAP_BLOCK_SIZE_BW);
    }

    unsafe fn add_free_block(&mut self, addr: u64, size: u64) {
        let _addr = self.head.start_addr();
        let block = FreeBlock {
            size,
            prev: Some(&mut *(_addr as *mut FreeBlock)),
            next: self.head.next.take(),
        };

        let mut _ptr = addr as *mut FreeBlock;
        _ptr.write(block);
        self.head.next = Some(&mut *_ptr);
        if let Some(ref mut next) = (*_ptr).next {
            next.prev = Some(&mut *_ptr);
        }
        (&mut *_ptr).write_addr_at_end();
    }

    fn expand_mask(&mut self, frame: UnusedPhysFrame<Size2MiB>, flags: PageTableFlags) {
        let s_ptr = self.mask.boudry_addr() as *mut u64;
        PAGE_TABLE
            .lock()
            .map_to(VirtAddr::new(self.mask.boudry_addr()), frame, flags);
        self.mask.size += 8 * FRAME_SIZE;
        let e_ptr = (self.mask.boudry_addr() - 8) as *mut u64;
        unsafe {
            s_ptr.write(0);
            e_ptr.write(0);
        }
    }

    unsafe fn find_and_alloc(&mut self, size: u64) -> *mut u8 {
        let mut curr = &self.head;

        let set_mask = |addr: u64, size: u64, partial: bool, mask: &mut BitMask| {
            let s_off = (addr - HEAP_START_ADDR) >> HEAP_BLOCK_SIZE_BW;
            let e_off = (addr + size - HEAP_START_ADDR) >> HEAP_BLOCK_SIZE_BW;
            mask.set_on(s_off);
            mask.set_on(e_off - 1);
            if s_off > 0 && partial {
                mask.set_off(s_off - 1)
            }
        };

        while curr.next.is_some() && curr.size < size {
            curr = curr.next.as_ref().unwrap();
        }

        let _addr = curr.start_addr();
        let _block = &mut *(_addr as *mut FreeBlock);
        let mut _size = _block.size;
        while _size < size {
            self.expand();
            _size = (&*(_addr as *const FreeBlock)).size;
        }

        if _size == size {
            let ptr = _block.release();
            set_mask(ptr as u64, size, false, &mut self.mask);
            ptr
        } else {
            let ptr = _block.alloc(size);
            set_mask(ptr as u64, size, true, &mut self.mask);
            ptr
        }
    }

    fn size_align(layout: Layout) -> (u64, u64) {
        (
            ((layout.size() + 63) >> HEAP_BLOCK_SIZE_BW << HEAP_BLOCK_SIZE_BW) as u64,
            8,
        )
    }
}

unsafe impl GlobalAlloc for Locked<HeapAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, _) = HeapAllocator::size_align(layout);
        self.lock().find_and_alloc(size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = HeapAllocator::size_align(layout);
        self.lock().ins_merg_free_block(ptr as u64, size);
    }
}

pub fn init() {
    unsafe {
        ALLOCATOR.lock().mask.inner =
            Some(&mut *(HEAP_MASK_START_ADDR as *mut [u64; HEAP_MAX_BLOCKS as usize]));
        ALLOCATOR.lock().expand();
    }
}
