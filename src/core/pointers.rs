use std::mem::ManuallyDrop;
use std::ptr::{from_raw_parts, from_raw_parts_mut, metadata};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct PtrMetadata {
    opaque: [usize; 1]
}

impl PtrMetadata {
    pub const unsafe fn null() -> PtrMetadata {
        PtrMetadata {
            opaque: [0; 1]
        }
    }
    pub const fn is_null(&self) -> bool {
        self.opaque[0] == 0
    }
}

#[rustversion::nightly]
use core::intrinsics::transmute_unchecked;

#[inline(always)]
const unsafe fn transmute_unchecked_unsized<A, B>(src: A) -> B {
    #[repr(C)]
    union TransmuteInlined<A, B> {
        p1: ManuallyDrop<A>,
        p2: ManuallyDrop<B>
    }
    let f = TransmuteInlined::<A, B> {
        p1: ManuallyDrop::new(src)
    };
    ManuallyDrop::into_inner(f.p2)
}
#[inline(always)]
pub unsafe fn fat_to_thin<T: ?Sized>(ptr: *const T) -> *const u8 {
    ptr as *const u8
}
#[inline(always)]
pub unsafe fn fat_to_thin_mut<T: ?Sized>(ptr: *mut T) -> *mut u8 {
    ptr as *mut u8
}

#[cfg(not(debug_assertions))]
#[inline(always)]
pub unsafe fn fat_to_metadata<T: ?Sized>(ptr: *const T) -> PtrMetadata {
    transmute_unchecked(metadata(ptr))
}
#[cfg(debug_assertions)]
#[inline(always)]
pub fn fat_to_metadata<T: ?Sized>(ptr: *const T) -> PtrMetadata {
    use std::alloc::Layout;

    use crate::godot_debug;

    let metadata_before = metadata(ptr);
    if Layout::for_value(&metadata_before).size() == 0 {
        godot_debug!("passed pointer is missing metadata");
        panic!("passed pointer is missing metadata");
    }
    unsafe { transmute_unchecked(metadata_before) }
}

#[inline(always)]
pub unsafe fn thin_to_fat<T: ?Sized>(ptr: *const u8, metadata: PtrMetadata) -> *const T {
    let metadata = transmute_unchecked_unsized(metadata);
    from_raw_parts(ptr, metadata)
}
#[inline(always)]
pub unsafe fn thin_to_fat_mut<T: ?Sized>(ptr: *mut u8, metadata: PtrMetadata) -> *mut T {
    let metadata = transmute_unchecked_unsized(metadata);
    from_raw_parts_mut(ptr, metadata)
}
#[inline(always)]
pub const fn null<T: ?Sized>() -> *const T {
    unsafe {transmute_unchecked_unsized((0usize, 0usize, 0usize))}
}
#[inline(always)]
pub const fn null_mut<T: ?Sized>() -> *mut T {
    unsafe {transmute_unchecked_unsized((0usize, 0usize, 0usize))}
}