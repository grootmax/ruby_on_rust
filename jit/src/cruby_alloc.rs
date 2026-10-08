use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, AtomicPtr, AtomicUsize, Ordering};
use std::ptr;

type XMallocFn = unsafe extern "C" fn(usize) -> *mut std::ffi::c_void;
type XFreeFn = unsafe extern "C" fn(*mut std::ffi::c_void);
type XReallocFn = unsafe extern "C" fn(*mut std::ffi::c_void, usize) -> *mut std::ffi::c_void;
type XCallocFn = unsafe extern "C" fn(usize, usize) -> *mut std::ffi::c_void;
type GcAdjustFn = unsafe extern "C" fn(isize);

static XMALLOC_PTR: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(ptr::null_mut());
static XFREE_PTR: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(ptr::null_mut());
static XREALLOC_PTR: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(ptr::null_mut());
static XCALLOC_PTR: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(ptr::null_mut());
static GC_ADJUST_PTR: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(ptr::null_mut());

const SENTINEL: *mut std::ffi::c_void = 1 as *mut std::ffi::c_void;

pub static TEST_GC_ADJUSTED_MEMORY: AtomicIsize = AtomicIsize::new(0);

#[cfg(not(target_os = "macos"))]
const RTLD_DEFAULT_PTR: *mut std::ffi::c_void = ptr::null_mut();
#[cfg(target_os = "macos")]
const RTLD_DEFAULT_PTR: *mut std::ffi::c_void = -2isize as *mut std::ffi::c_void;

unsafe extern "C" {
    fn dlsym(handle: *mut std::ffi::c_void, symbol: *const std::os::raw::c_char) -> *mut std::ffi::c_void;
}

fn resolve_symbol(name: &[u8]) -> *mut std::ffi::c_void {
    unsafe {
        let sym = dlsym(RTLD_DEFAULT_PTR, name.as_ptr() as *const _);
        if sym.is_null() {
            SENTINEL
        } else {
            sym
        }
    }
}

fn get_xmalloc() -> Option<XMallocFn> {
    let mut p = XMALLOC_PTR.load(Ordering::Relaxed);
    if p.is_null() {
        p = resolve_symbol(b"ruby_xmalloc\0");
        XMALLOC_PTR.store(p, Ordering::Relaxed);
    }
    if p == SENTINEL {
        None
    } else {
        Some(unsafe { std::mem::transmute(p) })
    }
}

fn get_xfree() -> Option<XFreeFn> {
    let mut p = XFREE_PTR.load(Ordering::Relaxed);
    if p.is_null() {
        p = resolve_symbol(b"ruby_xfree\0");
        XFREE_PTR.store(p, Ordering::Relaxed);
    }
    if p == SENTINEL {
        None
    } else {
        Some(unsafe { std::mem::transmute(p) })
    }
}

fn get_xrealloc() -> Option<XReallocFn> {
    let mut p = XREALLOC_PTR.load(Ordering::Relaxed);
    if p.is_null() {
        p = resolve_symbol(b"ruby_xrealloc\0");
        XREALLOC_PTR.store(p, Ordering::Relaxed);
    }
    if p == SENTINEL {
        None
    } else {
        Some(unsafe { std::mem::transmute(p) })
    }
}

fn get_xcalloc() -> Option<XCallocFn> {
    let mut p = XCALLOC_PTR.load(Ordering::Relaxed);
    if p.is_null() {
        p = resolve_symbol(b"ruby_xcalloc\0");
        XCALLOC_PTR.store(p, Ordering::Relaxed);
    }
    if p == SENTINEL {
        None
    } else {
        Some(unsafe { std::mem::transmute(p) })
    }
}

fn get_gc_adjust() -> Option<GcAdjustFn> {
    let mut p = GC_ADJUST_PTR.load(Ordering::Relaxed);
    if p.is_null() {
        p = resolve_symbol(b"rb_gc_adjust_memory_usage\0");
        GC_ADJUST_PTR.store(p, Ordering::Relaxed);
    }
    if p == SENTINEL {
        None
    } else {
        Some(unsafe { std::mem::transmute(p) })
    }
}

pub unsafe fn rb_gc_adjust_memory_usage(diff: isize) {
    if let Some(f) = get_gc_adjust() {
        unsafe { f(diff); }
    } else {
        TEST_GC_ADJUSTED_MEMORY.fetch_add(diff, Ordering::SeqCst);
    }
}

/// A global allocator bridge that routes Rust heap allocations through CRuby's `ruby_xmalloc`
/// and `ruby_xfree` FFI imports.
pub struct CRubyGlobalAlloc {
    pub alloc_size: AtomicUsize,
}

impl CRubyGlobalAlloc {
    pub const fn new() -> Self {
        Self {
            alloc_size: AtomicUsize::new(0),
        }
    }
}

unsafe impl GlobalAlloc for CRubyGlobalAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_size.fetch_add(layout.size(), Ordering::SeqCst);
        if let Some(f) = get_xmalloc() {
            unsafe { f(layout.size()) as *mut u8 }
        } else {
            unsafe { System.alloc(layout) }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.alloc_size.fetch_sub(layout.size(), Ordering::SeqCst);
        if let Some(f) = get_xfree() {
            unsafe { f(ptr as *mut std::ffi::c_void) }
        } else {
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.alloc_size.fetch_add(layout.size(), Ordering::SeqCst);
        if let Some(f) = get_xcalloc() {
            unsafe { f(1, layout.size()) as *mut u8 }
        } else {
            unsafe { System.alloc_zeroed(layout) }
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if new_size > layout.size() {
            self.alloc_size.fetch_add(new_size - layout.size(), Ordering::SeqCst);
        } else if new_size < layout.size() {
            self.alloc_size.fetch_sub(layout.size() - new_size, Ordering::SeqCst);
        }
        if let Some(f) = get_xrealloc() {
            unsafe { f(ptr as *mut std::ffi::c_void, new_size) as *mut u8 }
        } else {
            unsafe { System.realloc(ptr, layout, new_size) }
        }
    }
}

/// RAII guard for unmanaged JIT virtual memory page allocations.
/// Notifies CRuby GC via `rb_gc_adjust_memory_usage` when memory is mapped or unmapped.
pub struct CRubyMemoryGuard {
    size: usize,
}

impl CRubyMemoryGuard {
    pub fn new(size: usize) -> Self {
        if size > 0 {
            unsafe {
                rb_gc_adjust_memory_usage(size as isize);
            }
        }
        Self { size }
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for CRubyMemoryGuard {
    fn drop(&mut self) {
        if self.size > 0 {
            unsafe {
                rb_gc_adjust_memory_usage(-(self.size as isize));
            }
            self.size = 0;
        }
    }
}

/// Helper function to shrink or drop guards from a `Vec<CRubyMemoryGuard>` by `bytes_to_remove`.
pub fn shrink_memory_guards(guards: &mut Vec<CRubyMemoryGuard>, mut bytes_to_remove: usize) {
    while bytes_to_remove > 0 && !guards.is_empty() {
        let last_idx = guards.len() - 1;
        if guards[last_idx].size <= bytes_to_remove {
            bytes_to_remove -= guards[last_idx].size;
            guards.pop(); // Drops CRubyMemoryGuard, adjusting memory down
        } else {
            guards[last_idx].size -= bytes_to_remove;
            unsafe {
                rb_gc_adjust_memory_usage(-(bytes_to_remove as isize));
            }
            bytes_to_remove = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cruby_memory_guard_raii() {
        let initial = TEST_GC_ADJUSTED_MEMORY.load(Ordering::SeqCst);
        {
            let _guard = CRubyMemoryGuard::new(2048);
            assert_eq!(TEST_GC_ADJUSTED_MEMORY.load(Ordering::SeqCst), initial + 2048);
        }
        assert_eq!(TEST_GC_ADJUSTED_MEMORY.load(Ordering::SeqCst), initial);
    }

    #[test]
    fn test_shrink_memory_guards() {
        let initial = TEST_GC_ADJUSTED_MEMORY.load(Ordering::SeqCst);
        let mut guards = vec![CRubyMemoryGuard::new(4096)];
        assert_eq!(TEST_GC_ADJUSTED_MEMORY.load(Ordering::SeqCst), initial + 4096);

        shrink_memory_guards(&mut guards, 2048);
        assert_eq!(TEST_GC_ADJUSTED_MEMORY.load(Ordering::SeqCst), initial + 2048);
        assert_eq!(guards.len(), 1);
        assert_eq!(guards[0].size(), 2048);

        shrink_memory_guards(&mut guards, 2048);
        assert_eq!(TEST_GC_ADJUSTED_MEMORY.load(Ordering::SeqCst), initial);
        assert!(guards.is_empty());
    }
}
