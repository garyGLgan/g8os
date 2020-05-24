use spin::{Mutex, MutexGuard};
use x86_64::VirtAddr;

pub struct Locked<T> {
    inner: Mutex<T>,
}

impl<T> Locked<T> {
    pub fn new(t: T) -> Self {
        Locked {
            inner: Mutex::new(t),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        self.inner.lock()
    }
}
