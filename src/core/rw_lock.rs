use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::null;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use parking_lot::lock_api::RawRwLock as IRawRwLock;
use parking_lot::RawRwLock;

thread_local! {
    pub(super) static GLOBAL_LOCKS_OF_THREAD: RefCell<Vec<Arc<AtomicBool>>> = RefCell::default();
}

pub struct RwLock<T: ?Sized> {
    lock: RawRwLock,
    poisoned: AtomicBool,
    global_lock: *const AtomicBool,
    data: UnsafeCell<T>,
}
impl<T: Debug> Debug for RwLock<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RwLock")
            //.field("lock", &self.lock)
            .field("poisoned", &self.poisoned)
            .field("data", &self.data).finish()
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> Self {
        Self {
            lock: RawRwLock::INIT,
            poisoned: AtomicBool::new(false),
            data: UnsafeCell::default(),
            global_lock: null()
        }
    }
}

pub struct RwLockReadGuard<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
    holds_lock: bool
}
pub struct RwLockWriteGuard<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
    holds_lock: bool
}

impl<'a, T: ?Sized> !Send for RwLockReadGuard<'a, T> {}
impl<'a, T: ?Sized> !Send for RwLockWriteGuard<'a, T> {}
impl<'a, 'b, T: ?Sized> !Send for RwLockReadReleaseGuard<'a, 'b, T> {}
impl<'a, 'b, T: ?Sized> !Send for RwLockWriteReleaseGuard<'a, 'b, T> {}

impl<'a, T: Debug> Debug for RwLockReadGuard<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RwLockReadGuard").field("lock", &self.lock).finish()
    }
}
impl<'a, T: Debug> Debug for RwLockWriteGuard<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RwLockWriteGuard").field("lock", &self.lock).finish()
    }
}

pub struct RwLockReadReleaseGuard<'a, 'b, T: ?Sized> {
    guard: &'b mut RwLockReadGuard<'a, T>
}
pub struct RwLockWriteReleaseGuard<'a, 'b, T: ?Sized> {
    guard: &'b mut RwLockWriteGuard<'a, T>
}

impl <'a, 'b, T: Debug> Debug for RwLockReadReleaseGuard<'a, 'b, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RwLockReadReleaseGuard").field("guard", &self.guard).finish()
    }
}
impl <'a, 'b, T: Debug> Debug for RwLockWriteReleaseGuard<'a, 'b, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RwLockWriteReleaseGuard").field("guard", &self.guard).finish()
    }
}

pub struct PoisonError<T: ?Sized> {
    guard: T
}
pub type LockResult<T> = Result<T, PoisonError<T>>;
pub type TryLockResult< T> = Result<T, TryLockError<T>>;

unsafe impl<T: ?Sized> Send for RwLock<T> {}
unsafe impl<T: ?Sized> Sync for RwLock<T> {}
pub enum TryLockError<T> {
    WouldBlock,
    Poisoned(PoisonError<T>)
}

impl<T> RwLock<T> {
    pub fn new(value: T) -> Self {
        RwLock {
            lock: RawRwLock::INIT,
            poisoned: AtomicBool::new(false),
            data: UnsafeCell::new(value),
            global_lock: null()
        }
    }
    pub fn new_uninit() -> RwLock<MaybeUninit<T>> {
        RwLock {
            lock: RawRwLock::INIT,
            data: UnsafeCell::new(MaybeUninit::uninit()),
            poisoned: AtomicBool::new(false),
            global_lock: null()
        }
    }
    pub fn new_with_flag(value: T, global_lock: *const AtomicBool) -> Self {
        RwLock {
            lock: RawRwLock::INIT,
            poisoned: AtomicBool::new(false),
            data: UnsafeCell::new(value),
            global_lock
        }
    }
    pub fn new_uninit_with_flag(global_lock: *const AtomicBool) -> RwLock<MaybeUninit<T>> {
        RwLock {
            lock: RawRwLock::INIT,
            data: UnsafeCell::new(MaybeUninit::uninit()),
            poisoned: AtomicBool::new(false),
            global_lock
        }
    }
    pub fn new_with_flag_auto(value: T) -> Self {
        RwLock {
            lock: RawRwLock::INIT,
            poisoned: AtomicBool::new(false),
            data: UnsafeCell::new(value),
            global_lock: GLOBAL_LOCKS_OF_THREAD.with_borrow(|x|
                x.last().map(|x| &raw const *x.as_ref()).unwrap_or(null())
            )
        }
    }
    pub fn new_uninit_with_flag_auto() -> RwLock<MaybeUninit<T>> {
        RwLock {
            lock: RawRwLock::INIT,
            data: UnsafeCell::new(MaybeUninit::uninit()),
            poisoned: AtomicBool::new(false),
            global_lock: GLOBAL_LOCKS_OF_THREAD.with_borrow(|x|
                x.last().map(|x| &raw const *x.as_ref()).unwrap_or(null())
            )
        }
    }
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}
impl<T> RwLock<MaybeUninit<T>> {
    // SAFETY: Make sure the object is initialized.
    pub unsafe fn assume_init(self) -> RwLock<T> {
        RwLock {
            lock: RawRwLock::INIT,
            data: UnsafeCell::new(self.data.into_inner().assume_init()),
            poisoned: AtomicBool::new(false),
            global_lock: self.global_lock
        }
    }
}

impl<T: ?Sized> RwLock<T> {
    #[inline(always)]
    pub const unsafe fn access(&self) -> *mut T {
        self.data.get()
    }
    #[inline]
    pub fn read<'a>(&'a self) -> LockResult<RwLockReadGuard<'a, T>> {
        let holds_lock = unsafe { self.global_lock.as_ref().map_or(true, |x| x.load(Relaxed)) };

        if holds_lock {
            self.lock.lock_shared();
        }

        let rw_lock_read_guard = RwLockReadGuard {
            lock: self,
            holds_lock
        };

        if self.poisoned.load(Relaxed) { Err(PoisonError { guard: rw_lock_read_guard }) } else { Ok(rw_lock_read_guard) }
    }
    #[inline]
    pub fn try_read<'a>(&'a self) -> TryLockResult<RwLockReadGuard<'a, T>> {
        let holds_lock = unsafe { self.global_lock.as_ref().map_or(true, |x| x.load(Relaxed)) };

        if !holds_lock || self.lock.try_lock_shared() {
            let rw_lock_read_guard = RwLockReadGuard {
                lock: self,
                holds_lock
            };

            if self.poisoned.load(Relaxed) { Err(TryLockError::Poisoned(PoisonError { guard: rw_lock_read_guard })) } else { Ok(rw_lock_read_guard) }
        } else {
            Err(TryLockError::WouldBlock)
        }
    }
    #[inline]
    pub fn write<'a>(&'a self) -> LockResult<RwLockWriteGuard<'a, T>> {
        let holds_lock = unsafe { self.global_lock.as_ref().map_or(true, |x| x.load(Relaxed)) };

        if holds_lock {
            self.lock.lock_exclusive();
        }

        let rw_lock_write_guard = RwLockWriteGuard {
            lock: self,
            holds_lock
        };

        if self.poisoned.load(Relaxed) { Err(PoisonError { guard: rw_lock_write_guard }) } else { Ok(rw_lock_write_guard) }
    }
    #[inline]
    pub fn try_write<'a>(&'a self) -> TryLockResult<RwLockWriteGuard<'a, T>> {
        let holds_lock = unsafe { self.global_lock.as_ref().map_or(true, |x| x.load(Relaxed)) };

        if !holds_lock || self.lock.try_lock_exclusive() {
            let rw_lock_write_guard = RwLockWriteGuard {
                lock: self,
                holds_lock
            };

            if self.poisoned.load(Relaxed) { Err(TryLockError::Poisoned(PoisonError { guard: rw_lock_write_guard })) } else { Ok(rw_lock_write_guard) }
        } else {
            Err(TryLockError::WouldBlock)
        }
    }
    #[inline(always)]
    pub const fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
    #[inline(always)]
    pub fn is_poisoned(&self) -> bool {
        self.poisoned.load(Relaxed)
    }
    #[inline(always)]
    pub fn clear_poison(&self) {
        self.poisoned.store(false, Relaxed);
    }
    #[inline(always)]
    pub unsafe fn set_global_lock(&mut self, global_lock: *const AtomicBool) {
        self.global_lock = global_lock;
    }
    #[inline(always)]
    pub unsafe fn lock_shared(&self) {
        self.lock.lock_shared();
    }
    #[inline(always)]
    pub unsafe fn lock_exclusive(&self) {
        self.lock.lock_exclusive();
    }
    #[inline(always)]
    pub unsafe fn unlock_shared(&self) {
        self.lock.unlock_shared();
    }
    #[inline(always)]
    pub unsafe fn unlock_exclusive(&self) {
        self.lock.unlock_exclusive  ();

        if std::thread::panicking() {
            self.poisoned.store(true, Relaxed);
        }
    }
}

impl<'a, T: ?Sized> RwLockReadGuard<'a, T> {
    pub fn guard_release<'b>(&'b mut self) -> RwLockReadReleaseGuard<'a, 'b, T> {
        if self.holds_lock {
            // SAFETY: Restored when dropping release guard
            unsafe { self.lock.lock.unlock_shared() };
        }
        RwLockReadReleaseGuard { guard: self }
    }
}
impl<'a, T: ?Sized> RwLockWriteGuard<'a, T> {
    pub fn guard_release<'b>(&'b mut self) -> RwLockWriteReleaseGuard<'a, 'b, T> {
        if self.holds_lock {
            // SAFETY: Restored when dropping release guard
            unsafe { self.lock.lock.unlock_exclusive() };
        }
        RwLockWriteReleaseGuard { guard: self }
    }
}

impl<'a, 'b, T: ?Sized> Drop for RwLockReadReleaseGuard<'a, 'b, T> {
    fn drop(&mut self) {
        if self.guard.holds_lock {
            unsafe { self.guard.lock.lock_shared() };
        }
    }
}
impl<'a, 'b, T: ?Sized> Drop for RwLockWriteReleaseGuard<'a, 'b, T> {
    fn drop(&mut self) {
        if self.guard.holds_lock {
            unsafe { self.guard.lock.lock_exclusive() };
        }
    }
}

impl<'a, T: ?Sized> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        if self.holds_lock {
            unsafe { self.lock.unlock_shared() };
        }
    }
}
impl<'a, T: ?Sized> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        if self.holds_lock {
            unsafe { self.lock.unlock_exclusive() };
        }
    }
}

impl<'a, T: ?Sized> Deref for RwLockReadGuard<'a, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}
impl<'a, T: ?Sized> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}
impl<'a, T: ?Sized> DerefMut for RwLockWriteGuard<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Borrow<T> for RwLockReadGuard<'a, T> {
    #[inline]
    fn borrow(&self) -> &T {
        unsafe {&*self.lock.data.get()}
    }
}
impl<'a, T: ?Sized> Borrow<T> for RwLockWriteGuard<'a, T> {
    #[inline]
    fn borrow(&self) -> &T {
        unsafe {&*self.lock.data.get()}
    }
}
impl<'a, T: ?Sized> BorrowMut<T> for RwLockWriteGuard<'a, T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        unsafe { &mut*self.lock.data.get() } 
    }
}
impl<T> PoisonError<T> {
    #[inline]
    pub fn get_ref(&self) -> &T {
        &self.guard
    }
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.guard
    }
    #[inline]
    pub fn into_inner(self) -> T {
        self.guard
    }
}

impl<T: ?Sized> Debug for PoisonError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PoisonError").finish()
    }
}