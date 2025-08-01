use std::mem::ManuallyDrop;
#[cfg(debug_assertions)]
use std::ptr::Pointee;
use std::ptr::{from_raw_parts, from_raw_parts_mut, metadata};

#[repr(transparent)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct PtrMetadata {
    opaque: [*const (); 1],
}

impl PtrMetadata {
    pub const unsafe fn null() -> PtrMetadata {
        PtrMetadata {
            opaque: [null(); 1],
        }
    }
    pub const fn is_null(&self) -> bool {
        self.opaque[0].is_null()
    }
}

use core::intrinsics::transmute_unchecked;

#[inline(always)]
const unsafe fn transmute_unchecked_unsized<A, B>(src: A) -> B {
    #[repr(C)]
    union TransmuteInlined<A, B> {
        p1: ManuallyDrop<A>,
        p2: ManuallyDrop<B>,
    }
    let f = TransmuteInlined::<A, B> {
        p1: ManuallyDrop::new(src),
    };
    ManuallyDrop::into_inner(f.p2)
}
#[inline(always)]
pub const fn fat_to_thin<T: ?Sized>(ptr: *const T) -> *const u8 {
    ptr as *const u8
}
#[inline(always)]
pub const fn fat_to_thin_mut<T: ?Sized>(ptr: *mut T) -> *mut u8 {
    ptr as *mut u8
}

#[cfg(not(debug_assertions))]
#[inline(always)]
pub const fn fat_to_metadata<T: ?Sized>(ptr: *const T) -> PtrMetadata {
    transmute_unchecked(metadata(ptr))
}
#[cfg(debug_assertions)]
#[inline(always)]
pub const fn fat_to_metadata<T: Pointee + ?Sized>(ptr: *const T) -> PtrMetadata {
    const {
        let sz = size_of::<<T as Pointee>::Metadata>();
        let al = align_of::<<T as Pointee>::Metadata>();
        assert!(sz != 0, "pointer is missing metadata");
        assert!(
            sz == size_of::<*const ()>(),
            "pointer metadata is of unsupported size"
        );
        assert!(
            al <= align_of::<*const ()>(),
            "pointer metadata is of unsupported alignment"
        );
    }
    unsafe { transmute_unchecked(metadata(ptr)) }
}

#[inline(always)]
pub const unsafe fn thin_to_fat<T: ?Sized>(ptr: *const u8, metadata: PtrMetadata) -> *const T {
    let metadata = transmute_unchecked_unsized(metadata);
    from_raw_parts(ptr, metadata)
}
#[inline(always)]
pub const unsafe fn thin_to_fat_mut<T: ?Sized>(ptr: *mut u8, metadata: PtrMetadata) -> *mut T {
    let metadata = transmute_unchecked_unsized(metadata);
    from_raw_parts_mut(ptr, metadata)
}
#[inline(always)]
pub const fn null<T: ?Sized>() -> *const T {
    unsafe { transmute_unchecked_unsized((std::ptr::null() as *const u8, PtrMetadata::null())) }
}
#[inline(always)]
pub const fn null_mut<T: ?Sized>() -> *mut T {
    unsafe { transmute_unchecked_unsized((std::ptr::null_mut() as *mut u8, PtrMetadata::null())) }
}
