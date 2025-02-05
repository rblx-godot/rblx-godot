use std::alloc::{Layout, LayoutError};
use std::cell::Cell;
use std::ptr::{slice_from_raw_parts_mut, NonNull};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use godot::global::godot_error;

use super::{AllocError, Allocator, Global};

const DEFAULT_PAGE_SIZE: usize = 4096;

struct PagedAllocatorHead<A: Allocator> {
    layout: Option<Layout>,
    alloc: A,
    pages: Vec<*mut u8>,
    available_pages: Vec<*mut u8>, // PAGE, PAGE INDEX
    page_size: usize,
}
#[derive(Clone)]
pub struct PagedAllocator<A: Allocator = Global> {
    head: Arc<Mutex<PagedAllocatorHead<A>>>,
}

unsafe impl<A: Allocator> Send for PagedAllocator<A> {}
unsafe impl<A: Allocator> Sync for PagedAllocator<A> {}

#[derive(Clone)]
pub struct LocalPagedAllocator<A: Allocator = Global> {
    head: Rc<Cell<PagedAllocatorHead<A>>>,
}

impl PagedAllocator {
    pub fn new() -> Self {
        Self::new_in(Global)
    }
    pub fn new_with_page_size(page_size: usize) -> Self {
        Self::new_with_page_size_in(page_size, Global)
    }
}
impl<A: Allocator> PagedAllocator<A> {
    pub fn new_in(alloc: A) -> Self {
        Self::new_with_page_size_in(DEFAULT_PAGE_SIZE, alloc)
    }
    pub fn new_with_page_size_in(page_size: usize, alloc: A) -> Self {
        Self {
            head: Arc::new(Mutex::new(PagedAllocatorHead {
                layout: None,
                alloc,
                pages: Vec::new(),
                available_pages: Vec::new(),
                page_size,
            })),
        }
    }
}

impl LocalPagedAllocator {
    pub fn new() -> Self {
        Self::new_in(Global)
    }
    pub fn new_with_page_size(page_size: usize) -> Self {
        Self::new_with_page_size_in(page_size, Global)
    }
}
impl<A: Allocator> LocalPagedAllocator<A> {
    pub fn new_in(alloc: A) -> Self {
        Self::new_with_page_size_in(DEFAULT_PAGE_SIZE, alloc)
    }
    pub fn new_with_page_size_in(page_size: usize, alloc: A) -> Self {
        Self {
            head: Rc::new(Cell::new(PagedAllocatorHead {
                layout: None,
                alloc,
                pages: Vec::new(),
                available_pages: Vec::new(),
                page_size,
            })),
        }
    }
}

#[inline]
fn create_page_layout(layout: Layout, page_size: usize) -> Result<Layout, LayoutError> {
    // Inlined from Layout::array()
    if layout.size() != 0
        && page_size
            > unsafe { (isize::MAX as usize + 1).unchecked_sub(layout.align()) } / layout.size()
    {
        return Layout::from_size_align(1, 0); // returns always Err(LayoutError)
    }
    let array_size = unsafe { layout.size().unchecked_mul(page_size) };
    unsafe {
        Ok(Layout::from_size_align_unchecked(
            array_size,
            layout.align(),
        ))
    }
}
#[inline(always)]
fn offset_in_page(page: *mut u8, layout: Layout, index: usize) -> *mut u8 {
    unsafe { page.byte_add(layout.size() * index) }
}

unsafe impl<A: Allocator> Allocator for PagedAllocator<A> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let head = self.head.lock();
        if head.is_err() {
            return Err(AllocError);
        }
        let mut head = unsafe { head.unwrap_unchecked() };
        if head.layout.is_none() {
            head.layout = Some(layout);
        } else {
            debug_assert!(*head.layout.as_ref().unwrap() == layout);
        }
        if let Some(avail_index) = head.available_pages.pop() {
            return Ok(unsafe {
                NonNull::new_unchecked(slice_from_raw_parts_mut(avail_index, layout.size()))
            });
        }
        let new_page = head
            .alloc
            .allocate(create_page_layout(layout, head.page_size).unwrap())?;
        for i in 0..head.page_size {
            head.available_pages
                .push(offset_in_page(new_page.as_ptr().cast(), layout, i));
        }
        head.pages.push(new_page.as_ptr().cast());
        if let Some(avail_index) = head.available_pages.pop() {
            return Ok(unsafe {
                NonNull::new_unchecked(slice_from_raw_parts_mut(avail_index, layout.size()))
            });
        }
        unreachable!()
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let mut head = self.head.lock().unwrap();
        debug_assert!(*head.layout.as_ref().unwrap() == layout);
        debug_assert!(!head.available_pages.contains(&ptr.as_ptr()));
        head.available_pages.push(ptr.as_ptr());
    }
}

unsafe impl<A: Allocator> Allocator for LocalPagedAllocator<A> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let head = unsafe { &mut *self.head.as_ptr() };
        if head.layout.is_none() {
            head.layout = Some(layout);
        } else {
            debug_assert!(*head.layout.as_ref().unwrap() == layout);
        }
        if let Some(avail_index) = head.available_pages.pop() {
            return Ok(unsafe {
                NonNull::new_unchecked(slice_from_raw_parts_mut(avail_index, layout.size()))
            });
        }
        let new_page = head
            .alloc
            .allocate(create_page_layout(layout, head.page_size).unwrap())?;
        for i in 0..head.page_size {
            head.available_pages
                .push(offset_in_page(new_page.as_ptr().cast(), layout, i));
        }
        head.pages.push(new_page.as_ptr().cast());
        if let Some(avail_index) = head.available_pages.pop() {
            return Ok(unsafe {
                NonNull::new_unchecked(slice_from_raw_parts_mut(avail_index, layout.size()))
            });
        }
        unreachable!()
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let head = self.head.as_ptr().as_mut().unwrap_unchecked();
        debug_assert!(*head.layout.as_ref().unwrap() == layout);
        debug_assert!(!head.available_pages.contains(&ptr.as_ptr()));
        head.available_pages.push(ptr.as_ptr());
    }
}

impl<A: Allocator> Drop for PagedAllocatorHead<A> {
    fn drop(&mut self) {
        if self.available_pages.len() != self.pages.len() * self.page_size {
            godot_error!(
                "PagedAllocatorHead::<A>: {} leaked pages at exit.",
                self.pages.len()
            );
            godot_error!("Failed to deallocate pages, pages still in use.");
            return; // Do not perform the unsafe allocation if pages remain in use
        }
        if self.layout.is_some() {
            let layout = create_page_layout(self.layout.unwrap(), self.page_size).unwrap();
            for page in &self.pages {
                unsafe { self.alloc.deallocate(NonNull::new_unchecked(*page), layout) };
            }
        }
    }
}
