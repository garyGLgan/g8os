use spin::{Mutex, MutexGuard};
use x86_64::VirtAddr;

pub struct Locked<T> {
    inner: Mutex<T>,
}

impl<A> Locked<A> {
    pub fn new(t: A) -> Self {
        Locked {
            inner: Mutex::new(t),
        }
    }

    pub fn lock(&self) -> MutexGuard<A> {
        self.inner.lock()
    }
}

pub struct SysBitArray<'a> {
    size: u64,
    head: &'a mut u64,
}

impl<'a> SysBitArray<'a> {
    pub fn new(addr: VirtAddr, size: u64) -> Self {
        let mut s: u64 = 0;

        while s < 0 {
            let ptr = (addr.as_u64() + s) as *mut u64;
            unsafe {
                ptr.write(0 as u64);
            }
            s += 64;
        }
        unsafe {
            BitArray {
                size,
                bits: &mut *(addr.as_u64() as *mut u64),
            }
        }
    }

    pub fn set(&mut self, pos: u64) {
        let p1 = pos >> 6;
        let p2 = pos & 0x3f


    }

}
