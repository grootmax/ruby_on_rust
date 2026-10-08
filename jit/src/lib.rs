//! Shared code between YJIT and ZJIT.
#![warn(unsafe_op_in_unsafe_fn)] // Adopt 2024 edition default when targeting 2021 editions

use std::sync::atomic::{AtomicUsize, Ordering};
use std::alloc::{GlobalAlloc, Layout, System};

/// Encapsulates atomic allocation tracking metrics.
#[derive(Debug, Default)]
pub struct AllocatorMetrics {
    allocated_bytes: AtomicUsize,
}

impl AllocatorMetrics {
    /// Creates a new `AllocatorMetrics` handle initialized to zero allocated bytes.
    pub const fn new() -> Self {
        Self {
            allocated_bytes: AtomicUsize::new(0),
        }
    }

    /// Records an allocation of `size` bytes.
    pub fn record_alloc(&self, size: usize) {
        self.allocated_bytes.fetch_add(size, Ordering::SeqCst);
    }

    /// Records a deallocation of `size` bytes.
    pub fn record_dealloc(&self, size: usize) {
        self.allocated_bytes.fetch_sub(size, Ordering::SeqCst);
    }

    /// Returns the current total allocated bytes.
    pub fn allocated_bytes(&self) -> usize {
        self.allocated_bytes.load(Ordering::SeqCst)
    }
}

/// Hardened JIT global allocator wrapper around standard `System` allocator.
pub struct JitAllocator {
    metrics: AllocatorMetrics,
}

impl JitAllocator {
    /// Creates a new `JitAllocator` instance.
    pub const fn new() -> Self {
        Self {
            metrics: AllocatorMetrics::new(),
        }
    }

    /// Exposes read-only queries to allocation metrics.
    pub fn metrics(&self) -> &AllocatorMetrics {
        &self.metrics
    }
}

impl Default for JitAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[global_allocator]
pub static GLOBAL_ALLOCATOR: JitAllocator = JitAllocator::new();

/// Helper function wrapping System::alloc call.
///
/// # Safety
///
/// `layout` must have non-zero size per System allocator contract.
unsafe fn system_alloc(layout: Layout) -> *mut u8 {
    // SAFETY: Delegate directly to System::alloc; caller satisfies safety preconditions.
    unsafe { System.alloc(layout) }
}

/// Helper function wrapping System::dealloc call.
///
/// # Safety
///
/// `ptr` must denote a block of memory currently allocated via `System`,
/// and `layout` must match the layout used to allocate that block.
unsafe fn system_dealloc(ptr: *mut u8, layout: Layout) {
    // SAFETY: Delegate directly to System::dealloc; caller satisfies safety preconditions.
    unsafe { System.dealloc(ptr, layout) }
}

/// Helper function wrapping System::alloc_zeroed call.
///
/// # Safety
///
/// `layout` must have non-zero size per System allocator contract.
unsafe fn system_alloc_zeroed(layout: Layout) -> *mut u8 {
    // SAFETY: Delegate directly to System::alloc_zeroed; caller satisfies safety preconditions.
    unsafe { System.alloc_zeroed(layout) }
}

/// Helper function wrapping System::realloc call.
///
/// # Safety
///
/// `ptr` must denote a block of memory currently allocated via `System`,
/// `layout` must match the layout used to allocate that block, and `new_size`
/// must be greater than zero without overflowing `isize::MAX`.
unsafe fn system_realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    // SAFETY: Delegate directly to System::realloc; caller satisfies safety preconditions.
    unsafe { System.realloc(ptr, layout, new_size) }
}

unsafe impl GlobalAlloc for JitAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        debug_assert!(layout.size() > 0, "allocation layout size must be greater than zero");
        self.metrics.record_alloc(layout.size());
        // SAFETY: `layout` satisfies System::alloc preconditions verified by caller and debug assertion.
        unsafe { system_alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        debug_assert!(!ptr.is_null(), "pointer to deallocate must not be null");
        debug_assert!(layout.size() > 0, "deallocation layout size must be greater than zero");
        self.metrics.record_dealloc(layout.size());
        // SAFETY: `ptr` and `layout` satisfy System::dealloc preconditions verified by caller and debug assertion.
        unsafe { system_dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        debug_assert!(layout.size() > 0, "allocation layout size must be greater than zero");
        self.metrics.record_alloc(layout.size());
        // SAFETY: `layout` satisfies System::alloc_zeroed preconditions verified by caller and debug assertion.
        unsafe { system_alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        debug_assert!(!ptr.is_null(), "pointer to reallocate must not be null");
        debug_assert!(new_size > 0, "new allocation size must be greater than zero");
        if new_size > layout.size() {
            self.metrics.record_alloc(new_size - layout.size());
        } else if new_size < layout.size() {
            self.metrics.record_dealloc(layout.size() - new_size);
        }
        // SAFETY: `ptr`, `layout`, and `new_size` satisfy System::realloc preconditions.
        unsafe { system_realloc(ptr, layout, new_size) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocator_metrics_tracking() {
        let metrics = AllocatorMetrics::new();
        assert_eq!(metrics.allocated_bytes(), 0);

        metrics.record_alloc(1024);
        assert_eq!(metrics.allocated_bytes(), 1024);

        metrics.record_alloc(512);
        assert_eq!(metrics.allocated_bytes(), 1536);

        metrics.record_dealloc(1024);
        assert_eq!(metrics.allocated_bytes(), 512);

        metrics.record_dealloc(512);
        assert_eq!(metrics.allocated_bytes(), 0);
    }

    #[test]
    fn test_jit_allocator_metrics_access() {
        let allocator = JitAllocator::new();
        assert_eq!(allocator.metrics().allocated_bytes(), 0);

        let layout = Layout::from_size_align(64, 8).unwrap();
        unsafe {
            let ptr = allocator.alloc(layout);
            assert!(!ptr.is_null());
            assert_eq!(allocator.metrics().allocated_bytes(), 64);

            let new_ptr = allocator.realloc(ptr, layout, 128);
            assert!(!new_ptr.is_null());
            assert_eq!(allocator.metrics().allocated_bytes(), 128);

            let new_layout = Layout::from_size_align(128, 8).unwrap();
            allocator.dealloc(new_ptr, new_layout);
            assert_eq!(allocator.metrics().allocated_bytes(), 0);
        }
    }

    #[test]
    fn test_jit_allocator_alloc_zeroed() {
        let allocator = JitAllocator::new();
        let layout = Layout::from_size_align(32, 4).unwrap();
        unsafe {
            let ptr = allocator.alloc_zeroed(layout);
            assert!(!ptr.is_null());
            assert_eq!(allocator.metrics().allocated_bytes(), 32);
            for i in 0..32 {
                assert_eq!(*ptr.add(i), 0);
            }
            allocator.dealloc(ptr, layout);
            assert_eq!(allocator.metrics().allocated_bytes(), 0);
        }
    }
}

