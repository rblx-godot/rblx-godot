use std::{
    alloc::{alloc, alloc_zeroed, dealloc, realloc, GlobalAlloc, Layout},
    ptr::{slice_from_raw_parts_mut, NonNull},
};

use super::{AllocError, Allocator};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
pub struct Global;

unsafe impl GlobalAlloc for Global {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        alloc(layout)
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        alloc_zeroed(layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        realloc(ptr, layout, new_size)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        dealloc(ptr, layout);
    }
}
unsafe impl Allocator for Global {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            Ok(NonNull::new_unchecked(slice_from_raw_parts_mut(
                alloc(layout),
                layout.size(),
            )))
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        dealloc(ptr.as_ptr(), layout)
    }
}
unsafe impl Send for Global {}
unsafe impl Sync for Global {}
