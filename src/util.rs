use spin::{Mutex, MutexGuard};

pub struct Locked<T> {
    inner: Mutex<T>,
}

impl<T> Locked<T> {
    pub const fn new(t: T) -> Self {
        Locked {
            inner: Mutex::new(t),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        self.inner.lock()
    }
}
