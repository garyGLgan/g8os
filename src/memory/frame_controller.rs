use crate::kernel_const::{
    BOOT_TMP_MMAP_BUFFER, FRAME_SIZE, FRAME_SIZE_BIT_WIDTH, MEMORY_RESERVED_BELOW,
};
use crate::println;
use lazy_static::lazy_static;
use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size2MiB, UnusedPhysFrame},
    PhysAddr,
};

lazy_static! {
    pub static ref FRAME_ALLOC: PhysFrameAllocator = {
        let mut alloc = PhysFrameAllocator::new();
        unsafe {
            let mmap = &mut *(BOOT_TMP_MMAP_BUFFER as *mut MemoryMapBuffer);
            alloc.init(&mut *mmap);
        }
        alloc
    };
}
#[repr(packed)]
struct MemoryMapItem {
    addr: PhysAddr,
    size: u64,
    flags: u32,
    ext_flags: u32,
}

#[repr(packed)]
pub struct MemoryMapBuffer {
    len: u16,
    items: [MemoryMapItem; 1024], //assume the memroy map item less than 1024
}

struct UnusedBlock {
    addr: PhysAddr,
    size: u64,
    next: u64,
}

impl UnusedBlock {
    fn start_addr(&self) -> u64 {
        self.addr.as_u64()
    }

    fn end_addr(&self) -> u64 {
        self.start_addr() + (self.size as u64) * FRAME_SIZE
    }
}

const NUMBER_OF_FREE_BLOCK: u64 = 1024;
const NUMBER_OF_FREE_BLOCK_BIT_WIDTH: u64 = 10;
const NUMBER_OF_FREE_MASK: u64 = 1023;

pub struct PhysFrameAllocator {
    blocks: [Option<UnusedBlock>; 1024],
    start: u64,
    end: u64,
}

impl PhysFrameAllocator {
    pub fn new() -> Self {
        let blocks = [None; NUMBER_OF_FREE_BLOCK as usize];
        PhysFrameAllocator {
            blocks,
            start: 0,
            end: 0,
        }
    }

    pub fn init(&mut self, mmap: &mut MemoryMapBuffer) {
        for i in 0..(mmap.len as usize) {
            let item = &mmap.items[i];

            if item.size >= FRAME_SIZE && (item.size + item.addr.as_u64()) > MEMORY_RESERVED_BELOW {
                if item.addr.as_u64() < MEMORY_RESERVED_BELOW {
                    let _addr = PhysAddr::new(MEMORY_RESERVED_BELOW);
                    let _size = item.size + item.addr.as_u64() - MEMORY_RESERVED_BELOW;
                    if _size > FRAME_SIZE {
                        self.add_free_block(_addr, _size);
                    }
                } else {
                    self.add_free_block(item.addr, item.size);
                }
            }
        }
    }

    pub fn next(&mut self) -> u64 {
        assert!(self.end - self.start < NUMBER_OF_FREE_BLOCK);
        self.end += 1;
        self.end & NUMBER_OF_FREE_MASK
    }

    pub fn add_free_block(&mut self, addr: PhysAddr, size: u64) {
        let mut _addr = addr.as_u64();
        let mut _size = size;
        if !is_align(_addr) {
            _addr = align_up(_addr);
            _size = align_down(_size + addr.as_u64() - _addr);
        }

        self.add_aligned_free_block(PhysAddr::new(_addr), _size);
    }

    pub fn add_aligned_free_block(&mut self, addr: PhysAddr, size: u64) {
        if self.blocks[self.start as usize].is_none() {
            let _next = self.next();
            self.blocks[self.start as usize] = Some(UnusedBlock {
                addr,
                size: size >> FRAME_SIZE_BIT_WIDTH,
                next: _next,
            });
        } else {
            let mut cur = self.start;
            let mut next = self.blocks[cur as usize].as_ref().unwrap().next;
            while let Some(ref block) = self.blocks[next as usize] {
                if block.start_addr() >= addr.as_u64() {
                    break;
                }
                cur = next;
                next = block.next;
            }

            if self.blocks[next as usize].is_none() {
                let _next = self.next();
                self.blocks[next as usize] = Some(UnusedBlock {
                    addr,
                    size: size >> FRAME_SIZE_BIT_WIDTH,
                    next: _next,
                });
            } else {
                let _curr_end = self.blocks[cur as usize].as_ref().unwrap().end_addr();
                let _curr_start = self.blocks[cur as usize].as_ref().unwrap().start_addr();
                let _next_start = self.blocks[next as usize].as_ref().unwrap().start_addr();
                let _next_end = self.blocks[next as usize].as_ref().unwrap().end_addr();
                if _next_start > addr.as_u64() + size {
                    if _curr_end >= addr.as_u64() {
                        let _size = (addr.as_u64() + size - _curr_start) >> FRAME_SIZE_BIT_WIDTH;
                        self.blocks[cur as usize].as_mut().unwrap().size = _size;
                    } else {
                        let _next = self.next();
                        self.blocks[_next as usize] = Some(UnusedBlock {
                            addr,
                            size: size >> FRAME_SIZE_BIT_WIDTH,
                            next: next,
                        });
                        self.blocks[cur as usize].as_mut().unwrap().next = _next;
                    }
                } else {
                    let _size = (_next_end - addr.as_u64()) >> FRAME_SIZE_BIT_WIDTH;
                    self.blocks[next as usize].as_mut().unwrap().addr = addr;
                    self.blocks[next as usize].as_mut().unwrap().size = _size;
                }
            }
        }
    }

    pub fn print_out(&self) {
        println!("frame allocatr start:{}, end:{}", self.start, self.end);
        let mut curr = self.start;
        while let Some(ref b) = self.blocks[curr as usize] {
            println!(
                "Unused Block[{}]:[ start:0x{:x}, size:{}, next:{} ]",
                curr,
                b.start_addr(),
                b.size,
                b.next,
            );
            curr = b.next;
        }
    }
}

fn align_up(addr: u64) -> u64 {
    (addr + FRAME_SIZE - 1) >> FRAME_SIZE_BIT_WIDTH << FRAME_SIZE_BIT_WIDTH
}

fn align_down(addr: u64) -> u64 {
    addr >> FRAME_SIZE_BIT_WIDTH << FRAME_SIZE_BIT_WIDTH
}

fn is_align(addr: u64) -> bool {
    align_down(addr) == addr
}

unsafe impl FrameAllocator<Size2MiB> for PhysFrameAllocator {
    fn allocate_frame(&mut self) -> Option<UnusedPhysFrame<Size2MiB>> {
        if let Some(ref mut block) = self.blocks[self.start as usize] {
            let addr = block.addr;
            if block.size >= 1 {
                block.addr = addr + FRAME_SIZE;
                block.size -= 1;
            } else {
                self.start = block.next;
            }
            unsafe { Some(UnusedPhysFrame::new(PhysFrame::containing_address(addr))) }
        } else {
            None
        }
    }
}
