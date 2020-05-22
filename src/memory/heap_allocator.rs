// use alloc::alloc::{GlobalAlloc, Layout};
use crate::memory::frame_controller::FRAME_ALLOC;
use spin::Mutex;

const HEAP_MAX_SIZE: u64 = 0x100000000;


struct BitMask<'a> {
    size: u64,
    inner: &'a mut[u64; HEAP_MAX_SIZE as usize],
}

impl<'a> BitMask<'a> {
    fn new (addr: u64, size: u64) -> Self {
        unsafe{
            Self{
                size: size,
                inner: &mut *(addr as *mut [u64; HEAP_MAX_SIZE as usize]),
            }
        }
    }

    fn set_on(&mut self, pos: u64) {
        let (p, m) = self.split_pos(pos);
        self.inner[p] = self.inner[p] | m;
    }

    fn set_off(&mut self, pos: u64) {
        let (p, m) = self.split_pos(pos);
        self.inner[p] = self.inner[p] & !m;  
    }

    fn split_pos(&self,pos: u64) -> (usize, u64){
        assert!(pos < self.size);
        ((pos >> 6) as usize, 1<<(63 - (pos & 0x3f)))
    }

    fn is_set(&self, pos: u64) -> bool {
        let (p, m) = self.split_pos(pos);
        (self.inner[p] & m) != 0
    }
}