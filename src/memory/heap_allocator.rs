use crate::kernel_const::FRAME_SIZE;
use crate::memory::frame_controller::FRAME_ALLOC;
use crate::memory::paging::g8_page_table::PAGE_TABLE;
use crate::println;
use crate::util::Locked;
use alloc::alloc::{GlobalAlloc, Layout};
use x86_64::{
    structures::paging::{mapper::MapToError, PageTableFlags, Size2MiB, UnusedPhysFrame},
    VirtAddr,
};
use crate::no_interrupt;
use spin::Mutex;
use lazy_static::lazy_static;

const HEAP_MAX_BLOCKS: u64 = 0x4000000; // max heap size 128G
const HEAP_BLOCK_SIZE: u64 = 64; // matches cache line
const HEAP_BLOCK_SIZE_BW: u64 = 6; // bit width of heap block size
const HEAP_MASK_START_ADDR: u64 = 0x20000000;
const HEAP_START_ADDR: u64 = 0x40000000;

#[global_allocator]
static ALLOCATOR: Locked<HeapAllocator> = Locked::new(HeapAllocator::new());

lazy_static!{
    static ref MASK: Mutex<BitMask> = unsafe{ 
        Mutex::new(BitMask {
                        size: 0,
                        inner: &mut *(HEAP_MASK_START_ADDR as *mut [u64; HEAP_MAX_BLOCKS as usize]),
                    })
        };
}


const MASK_64: [u64; 65] = [
                            0b0,
                            0b1000000000000000000000000000000000000000000000000000000000000000,
                            0b1100000000000000000000000000000000000000000000000000000000000000,
                            0b1110000000000000000000000000000000000000000000000000000000000000,
                            0b1111000000000000000000000000000000000000000000000000000000000000,
                            0b1111100000000000000000000000000000000000000000000000000000000000,
                            0b1111110000000000000000000000000000000000000000000000000000000000,
                            0b1111111000000000000000000000000000000000000000000000000000000000,
                            0b1111111100000000000000000000000000000000000000000000000000000000,
                            0b1111111110000000000000000000000000000000000000000000000000000000,
                            0b1111111111000000000000000000000000000000000000000000000000000000,
                            0b1111111111100000000000000000000000000000000000000000000000000000,
                            0b1111111111110000000000000000000000000000000000000000000000000000,
                            0b1111111111111000000000000000000000000000000000000000000000000000,
                            0b1111111111111100000000000000000000000000000000000000000000000000,
                            0b1111111111111110000000000000000000000000000000000000000000000000,
                            0b1111111111111111000000000000000000000000000000000000000000000000,
                            0b1111111111111111100000000000000000000000000000000000000000000000,
                            0b1111111111111111110000000000000000000000000000000000000000000000,
                            0b1111111111111111111000000000000000000000000000000000000000000000,
                            0b1111111111111111111100000000000000000000000000000000000000000000,
                            0b1111111111111111111110000000000000000000000000000000000000000000,
                            0b1111111111111111111111000000000000000000000000000000000000000000,
                            0b1111111111111111111111100000000000000000000000000000000000000000,
                            0b1111111111111111111111110000000000000000000000000000000000000000,
                            0b1111111111111111111111111000000000000000000000000000000000000000,
                            0b1111111111111111111111111100000000000000000000000000000000000000,
                            0b1111111111111111111111111110000000000000000000000000000000000000,
                            0b1111111111111111111111111111000000000000000000000000000000000000,
                            0b1111111111111111111111111111100000000000000000000000000000000000,
                            0b1111111111111111111111111111110000000000000000000000000000000000,
                            0b1111111111111111111111111111111000000000000000000000000000000000,
                            0b1111111111111111111111111111111100000000000000000000000000000000,
                            0b1111111111111111111111111111111110000000000000000000000000000000,
                            0b1111111111111111111111111111111111000000000000000000000000000000,
                            0b1111111111111111111111111111111111100000000000000000000000000000,
                            0b1111111111111111111111111111111111110000000000000000000000000000,
                            0b1111111111111111111111111111111111111000000000000000000000000000,
                            0b1111111111111111111111111111111111111100000000000000000000000000,
                            0b1111111111111111111111111111111111111110000000000000000000000000,
                            0b1111111111111111111111111111111111111111000000000000000000000000,
                            0b1111111111111111111111111111111111111111100000000000000000000000,
                            0b1111111111111111111111111111111111111111110000000000000000000000,
                            0b1111111111111111111111111111111111111111111000000000000000000000,
                            0b1111111111111111111111111111111111111111111100000000000000000000,
                            0b1111111111111111111111111111111111111111111110000000000000000000,
                            0b1111111111111111111111111111111111111111111111000000000000000000,
                            0b1111111111111111111111111111111111111111111111100000000000000000,
                            0b1111111111111111111111111111111111111111111111110000000000000000,
                            0b1111111111111111111111111111111111111111111111111000000000000000,
                            0b1111111111111111111111111111111111111111111111111100000000000000,
                            0b1111111111111111111111111111111111111111111111111110000000000000,
                            0b1111111111111111111111111111111111111111111111111111000000000000,
                            0b1111111111111111111111111111111111111111111111111111100000000000,
                            0b1111111111111111111111111111111111111111111111111111110000000000,
                            0b1111111111111111111111111111111111111111111111111111111000000000,
                            0b1111111111111111111111111111111111111111111111111111111100000000,
                            0b1111111111111111111111111111111111111111111111111111111110000000,
                            0b1111111111111111111111111111111111111111111111111111111111000000,
                            0b1111111111111111111111111111111111111111111111111111111111100000,
                            0b1111111111111111111111111111111111111111111111111111111111110000,
                            0b1111111111111111111111111111111111111111111111111111111111111000,
                            0b1111111111111111111111111111111111111111111111111111111111111100,
                            0b1111111111111111111111111111111111111111111111111111111111111110,
                            0b1111111111111111111111111111111111111111111111111111111111111111,
                        ];

struct BitMask {
    size: u64,
    inner: &'static mut [u64; HEAP_MAX_BLOCKS as usize],
}

impl BitMask {
    fn boudry_addr(&self) -> u64 {
        HEAP_MASK_START_ADDR + (self.size >> 3)
    }

    fn split_pos(&self, pos: u64) -> (u64, u8) {
        assert!(pos < self.size);
        (pos >> 6, (pos & 0x3f) as u8)
    }

    unsafe fn is_set(&mut self, pos: u64) -> bool {
        let (p, m) = self.split_pos(pos);
        let _m = get_mask_in_u64(m, m);
        let v = get_u64(HEAP_MASK_START_ADDR+p*8);
        let r = v & _m != 0;
        println!("is_set, v:0x{:x}, _m:0x{:x}, r: {}", v, _m, r);
        r
    }

    fn is_empty(&self) -> bool {
        self.size == 0
    }

    unsafe fn range_off(&mut self, s: u64, e: u64) {
        self.set_by_u64(s, e, 
            |_s, _e, i| set_u64(HEAP_MASK_START_ADDR+i*8, get_u64(HEAP_MASK_START_ADDR+i*8)&!get_mask_in_u64( _s, _e)));
    }

    unsafe fn range_on(&mut self, s: u64, e: u64) {
        self.set_by_u64(s, e, 
            |_s, _e, i| set_u64(HEAP_MASK_START_ADDR+i*8, get_u64(HEAP_MASK_START_ADDR+i*8)|get_mask_in_u64(_s,_e)));
        
    }

    unsafe fn set_by_u64<F>(&mut self, s: u64, e: u64, f: F)
        where F: FnOnce(u8, u8, u64) -> () + Copy {
        assert!(s <= e);
        let (p1, q1) = self.split_pos(s);
        let (p2, q2) = self.split_pos(e);
        
        if p1 == p2 {
            f(q1, q2, p1 as u64);
        }else {
            for i in p1..=p2 {
                match i {
                    p1 => f(q1, 63, i as u64),
                    p2 => f(0, q2, i as u64),
                    _ => f(0, 63, i as u64),
                }
            }
        }
    }
}

unsafe fn get_u64(addr: u64) -> u64{
    (addr as *const u64).read()
}

unsafe fn set_u64( addr: u64, v: u64) {
    (addr as *mut u64).write(v);
}

fn get_mask_in_u64(s: u8, e: u8) -> u64{
    assert!(s>=0 && s<64);
    assert!(e>=0 && e<64);
    assert!(s<=e);
    !MASK_64[s as usize] & MASK_64[(e+1) as usize]
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
        println!("expand_backward, start:0x{:x}, end:0x{:x}, addr:0x{:x}", self.start_addr(), self.end_addr(), addr);
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
    head: FreeBlock,
    size: u64,
}

impl HeapAllocator {
    const fn new() -> Self {
        Self {
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
        println!("expand");
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
        println!("ins_merg_free_block, addr:0x{:x}, size:{}", addr, size);
        let s_off = (addr - HEAP_START_ADDR) >> HEAP_BLOCK_SIZE_BW;
        let e_off = (addr + size - HEAP_START_ADDR) >> HEAP_BLOCK_SIZE_BW;
        // println!("ins_merg_free_block");
        let mask_size = MASK.lock().size;
        let can_merge_pre = s_off > 0 && !MASK.lock().is_set(s_off - 1);
        let can_merge_back = e_off < mask_size && !MASK.lock().is_set(e_off);
        // println!("ins_merg_free_block, can_merge_pre:{}, can_merge_back:{}",can_merge_pre,can_merge_back);
        match (can_merge_pre, can_merge_back) {
            (true, true ) => {
                // println!("ins_merg_free_block, 1");
                let _addr = ((addr - 8) as *const u64).read();
                let _block = &mut *(_addr as *mut FreeBlock);
                _block.expand_backward(addr, size);
                _block.merge_next();
            },
            (true, false) => {
                // println!("ins_merg_free_block, 2");
                let _addr = ((addr - 8) as *const u64).read();
                let _block = &mut *(_addr as *mut FreeBlock);
                _block.expand_backward(addr, size);
            },
            (false, true) => {
                // println!("ins_merg_free_block, 3");
                let _block = &mut *((addr + size) as *mut FreeBlock);
                _block.expand_forward(addr, size);
            },
            (false, false) => {
                // println!("ins_merg_free_block, 4");
                self.add_free_block(addr, size);
            }
        }
        // println!("ins_merg_free_block, 5");

        MASK.lock().range_off(s_off, e_off-1);
    }

    unsafe fn add_free_block(&mut self, addr: u64, size: u64) {
        println!("add_free_block");
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
        println!("expand_mask");
        let s_off = MASK.lock().size;
        PAGE_TABLE
            .lock()
            .map_to(VirtAddr::new(MASK.lock().boudry_addr()), frame, flags);
            MASK.lock().size += 8 * FRAME_SIZE;
        let e_off = MASK.lock().size-1;
        unsafe {
            MASK.lock().range_off(s_off, e_off);
        }
    }

    unsafe fn find_and_alloc(&mut self, size: u64) -> *mut u8 {
        println!("find_and_alloc");
        let mut curr = &self.head;

        let set_mask = |addr: u64, size: u64| {
            let s_off = (addr - HEAP_START_ADDR) >> HEAP_BLOCK_SIZE_BW;
            let e_off = (addr + size - HEAP_START_ADDR -8) >> HEAP_BLOCK_SIZE_BW;
            MASK.lock().range_on(s_off, e_off);
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
            set_mask(ptr as u64, size);
            ptr
        } else {
            let ptr = _block.alloc(size);
            set_mask(ptr as u64, size);
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
        println!("alloc");
        let (size, _) = HeapAllocator::size_align(layout);
        self.lock().find_and_alloc(size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        println!("dealloc");
        let (size, _) = HeapAllocator::size_align(layout);
        self.lock().ins_merg_free_block(ptr as u64, size);
    }
}

pub fn init() {
    unsafe {
        ALLOCATOR.lock().expand();
    }
}
