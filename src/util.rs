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