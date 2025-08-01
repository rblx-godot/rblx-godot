use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

use parking_lot::{Mutex, MutexGuard};

pub struct Once<T: ?Sized = ()> {
    perm_lock: bool,
    locked: AtomicBool,
    mtx: Mutex<T>,
}

unsafe impl<T: ?Sized> Send for Once<T> where T: Send {}
unsafe impl<T: ?Sized> Sync for Once<T> where T: Sync {}

impl<T: Sized> Once<T> {
    pub const fn new(data: T) -> Self {
        Self {
            perm_lock: false,
            locked: AtomicBool::new(false),
            mtx: Mutex::new(data),
        }
    }

    pub fn acquire_write(&'_ self) -> Option<MutexGuard<'_, T>> {
        if self.perm_lock {
            None
        } else if self.locked.load(Relaxed) {
            None
        } else {
            Some(self.mtx.lock())
        }
    }

    pub fn acquire_read(&self) -> &T {
        if !self.locked.load(Relaxed) {
            let _guard = self.mtx.lock();
            self.locked.store(true, Relaxed);
        }
        unsafe { self.mtx.data_ptr().cast_const().as_ref().unwrap() }
    }
    pub fn lock(&mut self) {
        self.perm_lock = true;
    }
}
