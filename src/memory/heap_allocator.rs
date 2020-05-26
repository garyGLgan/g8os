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
const HEAP_BLOCK_SIZE: u64 = 64;        // matches cache line
const HEAP_BLOCK_SIZE_BW: u64 = 6;      // bit width of heap block size
const HEAP_MASK_START_ADDR: u64 = 0x20000000;
const HEAP_START_ADDR: u64 = 0x40000000;

struct FreeBlock<'a> {
    size: u64,
    prev: Option<&'a mut FreeBlock<'a>>,
    next: Option<&'a mut FreeBlock<'a>>,
}

impl <'a> FreeBlock<'a> {
    fn start_addr(&self) -> u64 {
        self as * const Self as u64
    }

    fn end_addr(&self) -> u64 {
        self.start_addr() + self.size
    }

    unsafe fn expand_backward(&mut self, addr: VirtAddr, size: u64) -> &mut Self {
        assert!(addr.as_u64() == self.end_addr());
        self.size += size;
        self.write_addr_at_end();
        self
    }

    unsafe fn expand_forward(&mut self, addr: VirtAddr, size: u64) -> &mut Self {
        assert!(addr.as_u64() + size == self.start_addr());

        let block = FreeBlock{
            size: self.size + size,
            prev: self.prev.take(),
            next: self.next.take(),
        };
        let _ptr = addr.as_u64() as *mut FreeBlock;
        _ptr.write(block);
        (&mut *_ptr).write_addr_at_end();
        if let Some(ref mut prev)=self.prev {
            prev.next = Some(&mut *_ptr);
        }
        if let Some(ref mut next)=self.next {
            next.prev = Some(&mut *_ptr);
        }
        (&mut *_ptr)
    }

    unsafe fn merge_next(&mut self) -> &mut Self {
        let s_addr = self.start_addr();
        let e_addr = self.end_addr();
        if let Some(ref mut next) = self.next {
            if e_addr == next.start_addr(){
                let block = &mut *(e_addr as *mut FreeBlock);
                self.size += block.size;
                match block.next {
                    None => self.next = None,
                    Some(ref mut b) => {
                        let _addr = b.start_addr();
                        self.next = Some(&mut *(_addr as *mut FreeBlock));
                        b.prev = Some(&mut *(self.start_addr() as *mut FreeBlock));
                    }
                }
                self.expand_backward(VirtAddr::new(block.start_addr()), block.size);
                self.write_addr_at_end();
            }
        }
        self
    }

    unsafe fn write_addr_at_end(&self) {
        let _e_ptr = (self.end_addr() -8) as *mut u64;
        _e_ptr.write(self as *const Self as u64);
    }
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
        for i in 0..(HEAP_BLOCK_SIZE * 8) {
            let (frame, flags) = alloc_frame()?;
            PAGE_TABLE
                .lock()
                .map_to(self.boundry_addr() + _size, frame, flags);
            _size += FRAME_SIZE;
        }
        self.size += _size;

        let (frame, flags) = alloc_frame()?;
        self.expand_mask(frame, flags);
        self.ins_merg_free_block(s_addr, _size);
        Ok(())
    }

    unsafe fn ins_merg_free_block(&mut self, addr: VirtAddr, size: u64) {
        let s_off = (addr.as_u64() - HEAP_START_ADDR)>>HEAP_BLOCK_SIZE_BW;
        let e_off = (addr.as_u64() + size - HEAP_START_ADDR)>>HEAP_BLOCK_SIZE_BW;
        let node = if s_off > 0 && !self.mask.is_set(s_off-1){ 
            let _addr = ((addr.as_u64() - 8) as *const u64).read();
            let _block =&mut *(_addr as *mut FreeBlock);
            _block.expand_backward(addr, size);
            if e_off < self.mask.size && !self.mask.is_set(e_off) {
                _block.merge_next();
            }
            _block
        } else if e_off < self.mask.size && !self.mask.is_set(e_off) {
            let _block = &mut *((addr.as_u64() + size) as *mut FreeBlock);
            _block.expand_forward(addr, size)
        } else {
            self.add_free_block(addr, size);
            self.head.next.as_mut().unwrap()
        };
        self.mask.set_off((node.start_addr()  - HEAP_START_ADDR)>>HEAP_BLOCK_SIZE_BW);
        self.mask.set_off((node.end_addr()  - HEAP_START_ADDR-1)>>HEAP_BLOCK_SIZE_BW);
    }

    unsafe fn add_free_block(&mut self, addr: VirtAddr, size: u64) {
            let mut block = FreeBlock{ 
                size,
                prev: None,
                next: self.head.next.take(),
            };
            
            let mut _ptr = addr.as_u64() as *mut FreeBlock;
            _ptr.write(block);
            self.head.next = Some(&mut *_ptr);
            if let Some(ref  mut next ) = (*_ptr).next {
                next.prev = Some(&mut *_ptr);
            }
            (&mut *_ptr).write_addr_at_end();
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

    fn is_empty(&self) -> bool {
        self.size == 0
    }
}
