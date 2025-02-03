use std::alloc::Layout;
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::mem::transmute;
use std::mem::ManuallyDrop;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::ptr::addr_eq;
use std::ptr::drop_in_place;
use std::ptr::write;
use std::ptr::NonNull;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::AcqRel;
use std::sync::atomic::Ordering::Acquire;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::Ordering::Release;

use crate::core::alloc::{Allocator, Global};
use crate::core::PtrMetadata;
use crate::core::RwLockReadReleaseGuard;
use crate::core::RwLockWriteReleaseGuard;
use crate::core::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::core::pointers::thin_to_fat_mut;

#[repr(C)]
struct TrcHeader<T: ?Sized>
{
    strong: AtomicU32,
    weak: AtomicU32,
    layout: Layout,
    lock: RwLock<ManuallyDrop<T>>
}
// important that T is always known, no downcasting will be allowed

// The object is cloned, so Send + Sync is not required
#[derive(Debug)]
pub struct Trc<T: ?Sized, A: Allocator = Global> {
    header: NonNull<TrcHeader<T>>,
    alloc: A
}
#[derive(Debug)]
pub struct Weak<T: ?Sized, A: Allocator = Global> {
    header: NonNull<TrcHeader<T>>,
    alloc: A
}

pub struct TrcReadLock<'a, T: ?Sized + 'static, A: Allocator = Global> {
    guard: RwLockReadGuard<'static, ManuallyDrop<T>>,
    rc: &'a Trc<T, A>
}
pub struct TrcWriteLock<'a, T: ?Sized + 'static, A: Allocator = Global> {
    guard: RwLockWriteGuard<'static, ManuallyDrop<T>>,
    rc: &'a Trc<T, A>
}

unsafe impl<T, A: Allocator> Send for Trc<T, A> {}
unsafe impl<T, A: Allocator> Sync for Trc<T, A> {}
unsafe impl<T, A: Allocator> Send for Weak<T, A> {}
unsafe impl<T, A: Allocator> Sync for Weak<T, A> {}

impl<T: RefUnwindSafe + ?Sized, A: Allocator + UnwindSafe> UnwindSafe for Trc<T, A> {}
impl<T: RefUnwindSafe + ?Sized, A: Allocator + UnwindSafe> RefUnwindSafe for Trc<T, A> {}
impl<T: RefUnwindSafe + ?Sized, A: Allocator + UnwindSafe> UnwindSafe for Weak<T, A> {}
impl<T: RefUnwindSafe + ?Sized, A: Allocator + UnwindSafe> RefUnwindSafe for Weak<T, A> {}

impl<T: ?Sized, A: Allocator + Clone> Clone for Trc<T, A> {
    fn clone(&self) -> Self {
        unsafe { self.increment_strong_count(); }
        Self { header: self.header.clone(), alloc: self.alloc.clone() }
    }
}
impl<T: ?Sized, A: Allocator + Clone> Clone for Weak<T, A> {
    fn clone(&self) -> Self {
        unsafe { self.increment_weak_count() };
        Self { header: self.header.clone(), alloc: self.alloc.clone() }
    }
}

impl<T: ?Sized, A: Allocator> Trc<T, A> {
    #[inline]
    fn inner(&self) -> &mut TrcHeader<T> {
        unsafe { self.header.as_ptr().as_mut().unwrap_unchecked() }
    }
    #[inline]
    pub unsafe fn access(&self) -> *mut T {
        self.inner().lock.access() as *mut T
    }
    
    // Uninitialized
    unsafe fn new_header(layout: Layout, alloc: &A) -> NonNull<TrcHeader<T>> {
        let (layout, _) = Layout::new::<TrcHeader<()>>().extend(layout).expect("layout error");
        let p = alloc.allocate(layout)
            .expect("memory allocation failed")
            .as_ptr();
        //let meta = metadata(p);
        let ptr: NonNull<TrcHeader<T>> = NonNull::new_unchecked(thin_to_fat_mut(p.cast::<u8>(), PtrMetadata::null()));
        let inner = ptr.as_ptr().as_mut().unwrap_unchecked();
        // It is important to use ptr::write on uninitialized fields.
        write(&raw mut inner.weak, AtomicU32::new(1));
        write(&raw mut inner.strong, AtomicU32::new(1));
        write(&raw mut inner.layout, layout);
        
        ptr
    }
    #[inline(always)]
    unsafe fn drop_header(&self) {
        drop_in_place(self.access());
    }
    #[inline(always)]
    unsafe fn free_header(&self) {
        self.alloc.deallocate(self.header.cast(), self.inner().layout);
    }
    
    // SAFETY: You must make sure the object is not dead and that it will be released back to prevent a memory leak!
    pub unsafe fn increment_strong_count(&self) {
        if self.inner().strong.fetch_add(1, Acquire) == 0 {
            self.inner().weak.fetch_add(1, Relaxed);
        }
    }
    unsafe fn increment_strong_count_if_exists(&self) -> bool {
        self.inner().strong.fetch_update(Release, Relaxed, |x| {
            if x == 0 {None}
            else {Some(x+1)}
        }).is_ok()
    }
    pub unsafe fn decrement_strong_count(&self) -> (bool, bool) {
        if self.inner().strong.fetch_sub(1, Acquire) == 1 {
            (true, self.inner().weak.fetch_sub(1, Relaxed) == 1)
        } else {
            (false, false)
        }
    }
    pub unsafe fn increment_weak_count(&self) {
        self.inner().weak.fetch_add(1, Relaxed);
    }
    pub unsafe fn decrement_weak_count(&self) -> bool {
        self.inner().weak.fetch_sub(1, Relaxed) == 1
    }
    pub fn strong_count(&self) -> u32 {
        self.inner().strong.load(Acquire)
    }
    pub fn weak_count(&self) -> u32 {
        self.inner().weak.load(Relaxed)
    }
}
impl<T> Trc<T> {
    #[inline]
    pub fn new(value: T) -> Self {
        Self::new_in(value, Global)
    }
    #[inline]
    pub fn new_cyclic<F>(data_fn: F) -> Self 
        where F: FnOnce(&Weak<T>) -> T
    {
        Self::new_cyclic_in(data_fn, Global)
    }
    #[inline]
    pub fn new_uninit() -> Trc<MaybeUninit<T>>
    {
        Self::new_uninit_in(Global)
    }
}
impl<T, A: Allocator> Trc<T, A> {
    pub fn new_in(value: T, alloc: A) -> Self {
        let head = unsafe { Self::new_header(Layout::new::<T>(), &alloc) };
        let this = Self {
            header: head,
            alloc
        };
        unsafe { 
            (&raw mut (head.as_ptr().as_mut().unwrap_unchecked().lock)).write(RwLock::new_with_flag_auto(ManuallyDrop::<T>::new(value)));
        }
        this
    }
    pub fn new_cyclic_in<F>(data_fn: F, alloc: A) -> Self 
    where 
        F: FnOnce(&Weak<T, A>) -> T,
        A: Clone
    {
        let head = unsafe { Self::new_header(Layout::new::<T>(), &alloc) };
        let this = Self {
            header: head,
            alloc
        };

        unsafe { 
            (&raw mut (head.as_ptr().as_mut().unwrap_unchecked().lock)).write(RwLock::new_with_flag_auto(ManuallyDrop::<T>::new(data_fn(&this.downgrade()))));
        }
        this
    }
    #[inline]
    pub fn new_uninit_in(alloc: A) -> Trc<MaybeUninit<T>, A>
    {
        let head = unsafe { Trc::<MaybeUninit<T>, A>::new_header(Layout::new::<MaybeUninit<T>>(), &alloc) };
        let this = Trc {
            header: head,
            alloc
        };
        unsafe { 
            (&raw mut (head.as_ptr().as_mut().unwrap_unchecked().lock)).write(RwLock::new_with_flag_auto(ManuallyDrop::new(MaybeUninit::uninit())));
        }
        this
    }
}
impl<T: ?Sized, A: Allocator + Clone> Trc<T, A> {
    pub fn downgrade(&self) -> Weak<T, A> {
        unsafe { self.increment_weak_count() };
        Weak { 
            header: self.header,
            alloc: self.alloc.clone()
        }
    }
}
impl<T: ?Sized, A: Allocator> Trc<T, A> {
    pub fn read<'a>(&'a self) -> TrcReadLock<'a, T, A> {
        debug_assert!(self.weak_count() != 0);
        debug_assert!(self.strong_count() != 0);
        TrcReadLock { 
            guard: unsafe { transmute(self.inner().lock.read().unwrap_or_else(|_|panic!("lock poisoned"))) },
            rc: &self
        }
    }
    pub fn write<'a>(&'a self) -> TrcWriteLock<'a, T, A> {
        debug_assert!(self.weak_count() != 0);
        debug_assert!(self.strong_count() != 0);
        TrcWriteLock { 
            guard: unsafe { transmute(self.inner().lock.write().unwrap_or_else(|_|panic!("lock poisoned"))) },
            rc: &self
        }
    }
}

impl<T: ?Sized, A: Allocator> Weak<T, A> {
    #[inline]
    fn inner(&self) -> &mut TrcHeader<T> {
        unsafe { self.header.as_ptr().as_mut().unwrap_unchecked() }
    }
    #[inline]
    unsafe fn access(&self) -> *mut T {
        self.inner().lock.access() as *mut T
    }
    #[inline(always)]
    unsafe fn drop_header(&self) {
        drop_in_place(self.access());
    }
    #[inline(always)]
    unsafe fn free_header(&self) {
        self.alloc.deallocate(self.header.cast(), self.inner().layout);
    }
    
    // SAFETY: You must make sure the object is not dead and that it will be released back to prevent a memory leak!
    pub unsafe fn increment_strong_count(&self) {
        if self.inner().strong.fetch_add(1, AcqRel) == 0 {
            self.inner().weak.fetch_add(1, AcqRel);
        }
    }
    unsafe fn increment_strong_count_if_exists(&self) -> bool {
        self.inner().strong.fetch_update(Release, Acquire, |x| {
            if x == 0 {None}
            else {Some(x+1)}
        }).is_ok()
    }
    pub unsafe fn decrement_strong_count(&self) -> (bool, bool) {
        if self.inner().strong.fetch_sub(1, Acquire) == 1 {
            (true, self.inner().weak.fetch_sub(1, Acquire) == 1)
        } else {
            (false, false)
        }
    }
    pub unsafe fn increment_weak_count(&self) {
        self.inner().weak.fetch_add(1, AcqRel);
    }
    pub unsafe fn decrement_weak_count(&self) -> bool {
        self.inner().weak.fetch_sub(1, AcqRel) == 1
    }
    pub fn strong_count(&self) -> u32 {
        self.inner().strong.load(Acquire)
    }
    pub fn weak_count(&self) -> u32 {
        self.inner().weak.load(Acquire)
    }
    pub fn dead(&self) -> bool {
        self.strong_count() == 0
    }
    pub fn upgrade(&self) -> Option<Trc<T, A>>
    where
        A: Clone
    {
        if unsafe { self.increment_strong_count_if_exists() } {
            Some(Trc {
                header: self.header,
                alloc: self.alloc.clone()
            })
        } else {
            None
        }
    }
}

impl<T: ?Sized, A: Allocator> Drop for Trc<T, A> {
    fn drop(&mut self) {
        unsafe {
            match self.decrement_strong_count() {
                (true, true) => {self.drop_header(); self.free_header();},
                (true, false) => self.drop_header(),
                _ => ()
            }
        }
    }
}
impl<T: ?Sized, A: Allocator> Drop for Weak<T, A> {
    fn drop(&mut self) {
        unsafe {
            match self.decrement_weak_count() {
                true => self.free_header(),
                _ => ()
            }
        }
    }
}

impl<'a, T: ?Sized, A: Allocator> Deref for TrcReadLock<'a, T, A> {
    type Target = T;

    fn deref(&self) -> &T {
        self.guard.deref()
    }
}
impl<'a, T: ?Sized, A: Allocator> Deref for TrcWriteLock<'a, T, A> {
    type Target = T;

    fn deref(&self) -> &T {
        self.guard.deref()
    }
}
impl<'a, T: ?Sized, A: Allocator> DerefMut for TrcWriteLock<'a, T, A> {
    fn deref_mut(&mut self) -> &mut T {
        self.guard.deref_mut()
    }
}
impl<'a, T: ?Sized, A: Allocator> Borrow<T> for TrcReadLock<'a, T, A> {
    fn borrow(&self) -> &T {
        self.guard.deref()
    }
}
impl<'a, T: ?Sized, A: Allocator> Borrow<T> for TrcWriteLock<'a, T, A> {
    fn borrow(&self) -> &T {
        self.guard.deref()
    }
}
impl<'a, T: ?Sized, A: Allocator> BorrowMut<T> for TrcWriteLock<'a, T, A> {
    fn borrow_mut(&mut self) -> &mut T {
        self.guard.deref_mut()
    }
}

impl<'a, T: ?Sized> TrcReadLock<'a, T> {
    pub fn guard_release<'b>(&'b mut self) -> RwLockReadReleaseGuard<'static, 'b, ManuallyDrop<T>> {
        self.guard.guard_release()
    }
}
impl<'a, T: ?Sized> TrcWriteLock<'a, T> {
    pub fn guard_release<'b>(&'b mut self) -> RwLockWriteReleaseGuard<'static, 'b, ManuallyDrop<T>> {
        self.guard.guard_release()
    }
}

impl<T: ?Sized, A: Allocator> PartialEq for Trc<T, A> {
    fn eq(&self, other: &Self) -> bool {
        addr_eq(self.header.as_ptr(), other.header.as_ptr())
    }
}

impl<T: Default, A: Allocator + Default> Default for Trc<T, A> {
    fn default() -> Self {
        Trc::new_in(T::default(), A::default())
    }
}