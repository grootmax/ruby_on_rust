//! Shared code between YJIT and ZJIT.
#![warn(unsafe_op_in_unsafe_fn)] // Adopt 2024 edition default when targeting 2021 editions

use std::sync::atomic::{AtomicUsize, Ordering};
use std::alloc::{GlobalAlloc, Layout, System};

#[global_allocator]
pub static GLOBAL_ALLOCATOR: StatsAlloc = StatsAlloc { alloc_size: AtomicUsize::new(0) };

pub struct StatsAlloc {
    pub alloc_size: AtomicUsize,
}

// SAFETY: StatsAlloc forwards all memory operations to System global allocator.
unsafe impl GlobalAlloc for StatsAlloc {
    // SAFETY: Caller must guarantee that layout has non-zero size per GlobalAlloc contract.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_size.fetch_add(layout.size(), Ordering::SeqCst);
        // SAFETY: Delegating allocation to System global allocator.
        unsafe { System.alloc(layout) }
    }

    // SAFETY: Caller must guarantee that ptr was allocated by this allocator with layout.
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.alloc_size.fetch_sub(layout.size(), Ordering::SeqCst);
        // SAFETY: Delegating deallocation to System global allocator.
        unsafe { System.dealloc(ptr, layout) }
    }

    // SAFETY: Caller must guarantee that layout has non-zero size per GlobalAlloc contract.
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.alloc_size.fetch_add(layout.size(), Ordering::SeqCst);
        // SAFETY: Delegating zeroed allocation to System global allocator.
        unsafe { System.alloc_zeroed(layout) }
    }

    // SAFETY: Caller must guarantee that ptr, layout, and new_size satisfy GlobalAlloc contract.
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if new_size > layout.size() {
            self.alloc_size.fetch_add(new_size - layout.size(), Ordering::SeqCst);
        } else if new_size < layout.size() {
            self.alloc_size.fetch_sub(layout.size() - new_size, Ordering::SeqCst);
        }
        // SAFETY: Delegating reallocation to System global allocator.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
