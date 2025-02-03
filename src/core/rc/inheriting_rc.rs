use std::alloc::Layout;
use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::mem::{forget, ManuallyDrop, MaybeUninit};
use std::ops::Deref;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::ptr::{addr_eq, NonNull};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::core::{fat_to_metadata, inheritance_cast_to_mut, inheritance_is_of_type, thin_to_fat_mut};
use crate::core::{alloc::{Allocator, Global}, null_mut, InheritanceBase};

fn create_layout_for_header(layout: Layout) -> Layout {
    Layout::new::<IrcHead<()>>().extend(layout).unwrap().0
}

#[repr(C)]
pub struct IrcHead<T: ?Sized> {
    layout: Layout, // represents T's size
    destroy: unsafe fn(*mut u8) -> (),
    strong: AtomicU32,
    weak: AtomicU32,
    base: *mut dyn InheritanceBase,
    data: ManuallyDrop<T>
}

impl<T: Sized + InheritanceBase> IrcHead<T> {
    fn new<A: Allocator>(value: T, alloc: &A) -> NonNull<Self> {
        let ptr = alloc.allocate(Layout::new::<Self>()).unwrap().cast();
        unsafe {
            ptr.write(IrcHead::<T> {
                layout: Layout::new::<T>(),
                destroy: |data| {
                    let t: *mut T = data as *mut T;
                    t.drop_in_place();
                },
                strong: AtomicU32::new(1),
                weak: AtomicU32::new(1),
                base: null_mut(),
                data: ManuallyDrop::new(value)
            });
            ptr.as_ptr().as_mut().unwrap_unchecked().base = (&raw mut ptr.as_ptr().as_mut().unwrap_unchecked().data) as *mut T as *mut dyn InheritanceBase;
        }
        ptr
    }
    fn new_cyclic<A, F>(data_fn: F, alloc: &A) -> NonNull<Self> 
    where 
        F: FnOnce (&IWeak<T, A>) -> T,
        A: Allocator + Clone
    {
        let mut ptr = alloc.allocate(Layout::new::<Self>()).unwrap().cast();
        let data_ptr;
        unsafe {
            ptr.write(IrcHead::<T> {
                layout: Layout::new::<T>(),
                destroy: |data| {
                    let t: *mut T = data as *mut T;
                    t.drop_in_place();
                },
                strong: AtomicU32::new(1),
                weak: AtomicU32::new(2),
                base: null_mut(),
                data: ManuallyDrop::new(MaybeUninit::uninit().assume_init())
            });
            ptr.as_ptr().as_mut().unwrap_unchecked().base = (&raw mut ptr.as_ptr().as_mut().unwrap_unchecked().data) as *mut T as *mut dyn InheritanceBase;
            
            data_ptr = (&raw mut ptr.as_mut().data).cast();
        }
        let weak = IWeak::<T, A> {
            head: ptr,
            ptr: data_ptr,
            alloc: alloc.clone()
        };
        unsafe {
            data_ptr.write(data_fn(&weak));
        }
        ptr
    }
    fn new_cyclic_fallable<A, F, E>(data_fn: F, alloc: &A) -> Result<NonNull<Self>, E>
    where 
        F: FnOnce (&IWeak<T, A>) -> Result<T, E>,
        A: Allocator + Clone
    {
        let mut ptr = alloc.allocate(Layout::new::<Self>()).unwrap().cast();
        let data_ptr;
        unsafe {
            ptr.write(IrcHead::<T> {
                layout: Layout::new::<T>(),
                destroy: |data| {
                    let t: *mut T = data as *mut T;
                    t.drop_in_place();
                },
                strong: AtomicU32::new(1),
                weak: AtomicU32::new(2),
                base: null_mut(),
                data: ManuallyDrop::new(MaybeUninit::uninit().assume_init())
            });
            ptr.as_ptr().as_mut().unwrap_unchecked().base = (&raw mut ptr.as_ptr().as_mut().unwrap_unchecked().data) as *mut T as *mut dyn InheritanceBase;
            
            data_ptr = (&raw mut ptr.as_mut().data).cast();
        }
        let weak = IWeak::<T, A> {
            head: ptr,
            ptr: data_ptr,
            alloc: alloc.clone()
        };
        unsafe {
            let val = data_fn(&weak);
            if val.is_err() {
                forget(weak);
                alloc.deallocate(ptr.cast(), Layout::new::<Self>());
                return Err(val.unwrap_err_unchecked());
            }
            data_ptr.write(val.unwrap_unchecked());
        }
        Ok(ptr)
    }
    fn new_uninit<A: Allocator>(alloc: &A) -> NonNull<IrcHead<MaybeUninit<T>>> {
        let ptr = alloc.allocate(Layout::new::<IrcHead<MaybeUninit<T>>>()).unwrap().cast();
        unsafe {
            ptr.write(IrcHead::<MaybeUninit<T>> {
                layout: Layout::new::<T>(),
                destroy: |data| {
                    let t: *mut T = data as *mut T;
                    t.drop_in_place();
                },
                strong: AtomicU32::new(1),
                weak: AtomicU32::new(1),
                base: null_mut(),
                data: ManuallyDrop::new(MaybeUninit::uninit())
            });
        }
        ptr
    }
}

impl<T: ?Sized> IrcHead<T> {
    unsafe fn drop_in_place(&mut self) {
        (self.destroy)((&raw mut self.data).cast())
    }
}
#[derive(Debug)]
pub struct Irc<T, A = Global>
where
    T: ?Sized,
    A: Allocator
{
    head: NonNull<IrcHead<T>>,
    ptr: *mut T,
    alloc: A
}


impl<T: RefUnwindSafe + ?Sized, A: Allocator + UnwindSafe> UnwindSafe for Irc<T, A> {}
impl<T: RefUnwindSafe + ?Sized, A: Allocator + UnwindSafe> RefUnwindSafe for Irc<T, A> {}
impl<T: RefUnwindSafe + ?Sized, A: Allocator + UnwindSafe> UnwindSafe for IWeak<T, A> {}
impl<T: RefUnwindSafe + ?Sized, A: Allocator + UnwindSafe> RefUnwindSafe for IWeak<T, A> {}

impl<T, A> Irc<T, A>
where
    T: ?Sized,
    A: Allocator
{
    fn deconstruct(self) -> (NonNull<IrcHead<T>>, *mut T, A) {
        let t = ManuallyDrop::new(self);
        (t.head, t.ptr, unsafe { (&raw const t.alloc).read() })
    }
    
    pub unsafe fn increment_strong_count(&self) {
        if self.head.as_ref().strong.fetch_add(1, Ordering::Acquire) == 0 {
            self.head.as_ref().weak.fetch_add(1, Ordering::Relaxed);
        }
    }
    pub unsafe fn decrement_strong_count(&self) -> (bool, bool) {
        if self.head.as_ref().strong.fetch_sub(1, Ordering::Acquire) == 1 {
            (true, self.head.as_ref().weak.fetch_sub(1, Ordering::Relaxed) == 1)
        } else {
            (false, false)
        }
    }
    unsafe fn increment_strong_count_if_exists(&self) -> bool {
        self.head.as_ref().strong.fetch_update(Ordering::Release, Ordering::Relaxed, |x| {
            if x == 0 {
                None
            } else {
                Some(x+1)
            }
        }).is_ok()
    }
    pub unsafe fn increment_weak_count(&self) {
        self.head.as_ref().weak.fetch_add(1, Ordering::Relaxed);
    }
    pub unsafe fn decrement_weak_count(&self) -> bool {
        self.head.as_ref().weak.fetch_sub(1, Ordering::Relaxed) == 1
    }
    pub fn strong_count(&self) -> u32 {
        unsafe { self.head.as_ref().strong.load(Ordering::Relaxed) }
    }
    pub fn weak_count(&self) -> u32 {
        unsafe { self.head.as_ref().weak.load(Ordering::Relaxed) }
    }
}

impl<T, A> IWeak<T, A>
where
    T: ?Sized,
    A: Allocator
{
    fn deconstruct(self) -> (NonNull<IrcHead<T>>, *mut T, A) {
        let t = ManuallyDrop::new(self);
        (t.head, t.ptr, unsafe { (&raw const t.alloc).read() })
    }
    
    unsafe fn increment_strong_count(&self) {
        if self.head.as_ref().strong.fetch_add(1, Ordering::Acquire) == 0 {
            self.head.as_ref().weak.fetch_add(1, Ordering::Relaxed);
        }
    }
    unsafe fn decrement_strong_count(&self) -> (bool, bool) {
        if self.head.as_ref().strong.fetch_sub(1, Ordering::Acquire) == 1 {
            (true, self.head.as_ref().weak.fetch_sub(1, Ordering::Relaxed) == 1)
        } else {
            (false, false)
        }
    }
    unsafe fn increment_strong_count_if_exists(&self) -> bool {
        self.head.as_ref().strong.fetch_update(Ordering::Release, Ordering::Relaxed, |x| {
            if x == 0 {
                None
            } else {
                Some(x+1)
            }
        }).is_ok()
    }
    unsafe fn increment_weak_count(&self) {
        self.head.as_ref().weak.fetch_add(1, Ordering::Relaxed);
    }
    unsafe fn decrement_weak_count(&self) -> bool {
        self.head.as_ref().weak.fetch_sub(1, Ordering::Relaxed) == 1
    }
    fn strong_count(&self) -> u32 {
        unsafe { self.head.as_ref().strong.load(Ordering::Relaxed) }
    }
    fn weak_count(&self) -> u32 {
        unsafe { self.head.as_ref().weak.load(Ordering::Relaxed) }
    }
}

impl<T> Irc<T>
where
    T: Sized + InheritanceBase,
{
    pub fn new(value: T) -> Self {
        let head = IrcHead::<T>::new(value, &Global);
        Irc::<T> {
            head,
            ptr: unsafe { &raw const head.as_ref().data }.cast_mut().cast(),
            alloc: Global
        }
    }
    pub fn new_cyclic<F>(data_fn: F) -> Self
    where
        F: FnOnce(&IWeak<T>) -> T
    {
        let head = IrcHead::<T>::new_cyclic(data_fn, &Global);
        Irc::<T> {
            head,
            ptr: unsafe { &raw const head.as_ref().data }.cast_mut().cast(),
            alloc: Global
        }
    }
    pub fn new_cyclic_fallable<F, E>(data_fn: F) -> Result<Self, E>
    where
        F: FnOnce(&IWeak<T>) -> Result<T, E>
    {
        let head = IrcHead::<T>::new_cyclic_fallable(data_fn, &Global)?;
        Ok(Irc::<T> {
            head,
            ptr: unsafe { &raw const head.as_ref().data }.cast_mut().cast(),
            alloc: Global
        })
    }
    pub fn new_uninit() -> Irc<MaybeUninit<T>> {
        let head = IrcHead::<T>::new_uninit( &Global);
        Irc::<MaybeUninit<T>> {
            head,
            ptr: unsafe { &raw const head.as_ref().data }.cast_mut().cast(),
            alloc: Global
        }
    }
}

impl<T, A> Irc<T, A>
where
    T: Sized + InheritanceBase,
    A: Allocator
{
    pub fn new_in(value: T, alloc: A) -> Self {
        let head = IrcHead::<T>::new(value, &alloc);
        Irc::<T, A> {
            head,
            ptr: unsafe { &raw const head.as_ref().data }.cast_mut().cast(),
            alloc
        }
    }
    pub fn new_cyclic_in<F>(data_fn: F, alloc: A) -> Self
    where
        F: FnOnce(&IWeak<T, A>) -> T,
        A: Clone
    {
        let head = IrcHead::<T>::new_cyclic(data_fn, &alloc);
        Irc::<T, A> {
            head,
            ptr: unsafe { &raw const head.as_ref().data }.cast_mut().cast(),
            alloc
        }
    }
    pub fn new_uninit_in(alloc: A) -> Irc<MaybeUninit<T>, A> {
        let head = IrcHead::<T>::new_uninit(&alloc);
        Irc::<MaybeUninit<T>, A> {
            head,
            ptr: unsafe { &raw const head.as_ref().data }.cast_mut().cast(),
            alloc
        }
    }
}


impl<T, A> Irc<T, A>
where
    T: InheritanceBase + ?Sized + 'static,
    A: Allocator
{
    pub fn cast_from_unsized<U: 'static + ?Sized>(self) -> Result<Irc<U, A>, Irc<T, A>> {
        let (head, ptr, alloc) = self.deconstruct();
        let p = head.as_ptr();
        let metadata = fat_to_metadata(p); 
        let base =  unsafe { head.as_ptr().as_mut().unwrap_unchecked().base.as_mut().unwrap_unchecked() };
        let result = inheritance_cast_to_mut!(base, U);
        if result.is_ok() {
            unsafe {
                let new_ptr = &raw mut *result.unwrap_unchecked();
                Ok(Irc::<U, A> {
                    head: NonNull::new_unchecked(thin_to_fat_mut(head.cast().as_ptr(), metadata)),
                    ptr: new_ptr,
                    alloc
                })
            }
        } else {
            Err(Irc::<T, A> {
                head,
                ptr,
                alloc
            })
        }
    }
    pub fn is<U: 'static + ?Sized>(&self) -> bool {
        let base =  unsafe { self.head.as_ref().base.as_ref().unwrap_unchecked() };
        inheritance_is_of_type!(base, U)
    }
}
impl<T, A> Irc<T, A>
where
    T: InheritanceBase + 'static,
    A: Allocator
{
    pub fn cast_from_sized<U: 'static + ?Sized>(self) -> Result<Irc<U, A>, Irc<T, A>> {
        let (head, ptr, alloc) = self.deconstruct();
        let p = head.as_ptr();
        let metadata = fat_to_metadata(p as *mut IrcHead<dyn InheritanceBase>); 
        let base =  unsafe { head.as_ptr().as_mut().unwrap_unchecked().base.as_mut().unwrap_unchecked() };
        let result = inheritance_cast_to_mut!(base, U);
        if result.is_ok() {
            unsafe {
                let new_ptr = &raw mut *result.unwrap_unchecked();
                Ok(Irc::<U, A> {
                    head: NonNull::new_unchecked(thin_to_fat_mut(head.cast().as_ptr(), metadata)),
                    ptr: new_ptr,
                    alloc
                })
            }
        } else {
            Err(Irc::<T, A> {
                head,
                ptr,
                alloc
            })
        }
    }
}

impl<T, A> Irc<MaybeUninit<T>, A>
where
    T: InheritanceBase,
    A: Allocator
{
    pub unsafe fn assume_init(self) -> Irc<T, A> {
        let (head, ptr, alloc) = self.deconstruct();
        let ptr = ptr.cast();
        head.as_ptr().as_mut().unwrap_unchecked().base = ptr as *mut dyn InheritanceBase;
        Irc::<T, A> {
            head: head.cast(),
            ptr,
            alloc
        }
    }
}

impl<T, A> Irc<T, A>
where
    T: ?Sized,
    A: Allocator
{
    pub unsafe fn access(&self) -> *mut T {
        self.ptr
    }
}
impl<T, A> Deref for Irc<T, A>
where
    T: ?Sized,
    A: Allocator
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref().unwrap_unchecked() }
    }
}
impl<T, A> Borrow<T> for Irc<T, A>
where
    T: ?Sized,
    A: Allocator
{
    fn borrow(&self) -> &T {
        unsafe { self.ptr.as_ref().unwrap_unchecked() }
    }
}

impl<T, A> Irc<T, A>
where
    T: ?Sized,
    A: Allocator + Clone
{
    pub fn downgrade(&self) -> IWeak<T, A> {
        unsafe { self.increment_weak_count(); }
        IWeak { head: self.head, ptr: self.ptr, alloc: self.alloc.clone() }
    }
}

#[derive(Debug)]
pub struct IWeak<T, A = Global>
where
    T: ?Sized,
    A: Allocator
{
    head: NonNull<IrcHead<T>>,
    ptr: *mut T,
    alloc: A
}

unsafe impl<T, A> Send for Irc<T, A> where
    T: ?Sized + Send + Sync,
    A: Allocator + Send + Sync
{}
unsafe impl<T, A> Sync for Irc<T, A> where
    T: ?Sized + Send + Sync,
    A: Allocator + Send + Sync
{}

unsafe impl<T, A> Send for IWeak<T, A> where
    T: ?Sized + Send + Sync,
    A: Allocator + Send + Sync
{}
unsafe impl<T, A> Sync for IWeak<T, A> where
    T: ?Sized + Send + Sync,
    A: Allocator + Send + Sync
{}


impl<T, A> Drop for Irc<T, A>
where
    T: ?Sized,
    A: Allocator
{
    #[inline]
    fn drop(&mut self) {
        let head = unsafe {
            self.head.as_mut()
        };
        match unsafe {self.decrement_strong_count()}  {
            (true, false) => unsafe { head.drop_in_place(); },
            (true, true) => unsafe {
                head.drop_in_place();
                self.alloc.deallocate(
                    self.head.cast(),
                    create_layout_for_header(head.layout)
                );
            },
            _ => ()
        }
    }
}

impl<T, A> Drop for IWeak<T, A>
where
    T: ?Sized,
    A: Allocator
{
    #[inline]
    fn drop(&mut self) {
        let head = unsafe { self.head.as_mut() };
        if unsafe { self.decrement_weak_count() } {
            unsafe { self.alloc.deallocate(
                self.head.cast(),
                create_layout_for_header(head.layout)
            ) };
        }
    }
}

impl<T, A> IWeak<T, A>
where
    T: ?Sized,
    A: Allocator + Clone
{
    pub fn upgrade(&self) -> Option<Irc<T, A>> {
        if unsafe { self.increment_strong_count_if_exists() } {
            Some(Irc::<T, A> {
                head: self.head,
                ptr: self.ptr,
                alloc: self.alloc.clone()
            })
        } else {
            None
        }
    }
    pub fn dead(&self) -> bool {
        self.weak_count() == 0
    }
    pub fn into_inner_with_allocator(self) -> (NonNull<IrcHead<T>>, *mut T, A) {
        let m = ManuallyDrop::new(self);
        let a = unsafe { (&raw const m.alloc).read() };
        let p = m.ptr;
        let head = m.head;
        (head, p, a)
    }
    pub fn from_inner_with_allocator(tuple: (NonNull<IrcHead<T>>, *mut T, A)) -> Self {
        IWeak {
            head: tuple.0,
            ptr: tuple.1,
            alloc: tuple.2
        }
    }
}

impl <T, A> Clone for Irc<T, A>
where
    T: ?Sized,
    A: Allocator + Clone
{
    fn clone(&self) -> Self {
        unsafe { self.increment_strong_count(); }
        Self {
            head: self.head,
            ptr: self.ptr,
            alloc: self.alloc.clone()
        }
    }
}
impl <T, A> Clone for IWeak<T, A>
where
    T: ?Sized,
    A: Allocator + Clone
{
    fn clone(&self) -> Self {
        unsafe { self.increment_weak_count(); }
        Self {
            head: self.head,
            ptr: self.ptr,
            alloc: self.alloc.clone()
        }
    }
}

impl <T, A> Hash for Irc<T, A>
where
    T: ?Sized,
    A: Allocator
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.head.hash(state);
    }
}

impl <T, A> Hash for IWeak<T, A>
where
    T: ?Sized,
    A: Allocator
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.head.hash(state);
    }
}


impl <T, A> PartialEq for Irc<T, A>
where
    T: ?Sized,
    A: Allocator
{
    fn eq(&self, other: &Self) -> bool {
        addr_eq(self.head.as_ptr(), other.head.as_ptr())
    }
}

impl <T, A> PartialEq for IWeak<T, A>
where
    T: ?Sized,
    A: Allocator
{
    fn eq(&self, other: &Self) -> bool {
        addr_eq(self.head.as_ptr(), other.head.as_ptr())
    }

}

impl <T: ?Sized, A: Allocator> Eq for Irc<T, A> {}
impl <T: ?Sized, A: Allocator> Eq for IWeak<T, A> {}