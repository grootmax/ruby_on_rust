//! Shared code between YJIT and ZJIT.
#![warn(unsafe_op_in_unsafe_fn)] // Adopt 2024 edition default when targeting 2021 editions

use std::sync::atomic::{AtomicUsize, Ordering};
use std::alloc::{GlobalAlloc, Layout, System};

#[global_allocator]
pub static GLOBAL_ALLOCATOR: StatsAlloc = StatsAlloc { alloc_size: AtomicUsize::new(0) };

#[allow(unsafe_op_in_unsafe_fn)]
#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(clippy::all)]
pub mod cruby {
    #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
    #[repr(transparent)]
    pub struct VALUE(pub usize);

    #[repr(C)]
    pub struct rb_control_frame_struct {
        _data: [u8; 0],
        _marker: std::marker::PhantomData<(*mut u8, std::marker::PhantomPinned)>,
    }

    #[repr(C)]
    pub struct rb_execution_context_struct {
        _data: [u8; 0],
        _marker: std::marker::PhantomData<(*mut u8, std::marker::PhantomPinned)>,
    }
    pub type rb_execution_context_t = rb_execution_context_struct;

    #[repr(C)]
    pub struct rb_method_definition_t {
        _data: [u8; 0],
        _marker: std::marker::PhantomData<(*mut u8, std::marker::PhantomPinned)>,
    }
    pub type rb_method_definition_struct = rb_method_definition_t;

    #[repr(C)]
    pub struct rb_method_cfunc_t {
        _data: [u8; 0],
        _marker: std::marker::PhantomData<(*mut u8, std::marker::PhantomPinned)>,
    }

    #[repr(C)]
    pub struct rb_callcache {
        _data: [u8; 0],
        _marker: std::marker::PhantomData<(*mut u8, std::marker::PhantomPinned)>,
    }

    #[repr(C)]
    pub struct rb_cref_t {
        _data: [u8; 0],
        _marker: std::marker::PhantomData<(*mut u8, std::marker::PhantomPinned)>,
    }

    #[repr(C)]
    pub struct rb_iseq_t {
        _data: [u8; 0],
        _marker: std::marker::PhantomData<(*mut u8, std::marker::PhantomPinned)>,
    }

    include!("cruby_bindings.inc.rs");
}

pub struct StatsAlloc {
    pub alloc_size: AtomicUsize,
}

unsafe impl GlobalAlloc for StatsAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_size.fetch_add(layout.size(), Ordering::SeqCst);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.alloc_size.fetch_sub(layout.size(), Ordering::SeqCst);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.alloc_size.fetch_add(layout.size(), Ordering::SeqCst);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if new_size > layout.size() {
            self.alloc_size.fetch_add(new_size - layout.size(), Ordering::SeqCst);
        } else if new_size < layout.size() {
            self.alloc_size.fetch_sub(layout.size() - new_size, Ordering::SeqCst);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
