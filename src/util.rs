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


pub struct Flag(bool);

impl Flag{
    pub const fn new() -> Self{
        Flag(false)
    }

    pub fn on(&mut self) {
        self.0 = true;
    }

    pub fn off(&mut self) {
       self.0 = false;
    }

    pub fn get(&self) -> bool{
        self.0
    }
}