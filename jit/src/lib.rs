//! Shared code between YJIT and ZJIT.
#![warn(unsafe_op_in_unsafe_fn)] // Adopt 2024 edition default when targeting 2021 editions
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(transparent)]
pub struct VALUE(pub usize);

#[repr(C)]
pub struct rb_iseq_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct rb_execution_context_struct {
    _private: [u8; 0],
}
pub type rb_execution_context_t = rb_execution_context_struct;

#[repr(C)]
pub struct rb_method_definition_struct {
    _private: [u8; 0],
}
pub type rb_method_definition_t = rb_method_definition_struct;

#[repr(C)]
pub struct rb_method_cfunc_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct rb_callcache {
    _private: [u8; 0],
}

#[repr(C)]
pub struct rb_control_frame_struct {
    _private: [u8; 0],
}

#[repr(C)]
pub struct rb_cref_t {
    _private: [u8; 0],
}

#[allow(dead_code, non_snake_case, non_camel_case_types, non_upper_case_globals, unused_imports, unsafe_op_in_unsafe_fn, clippy::all)]
pub mod cruby {
    use super::*;
    include!("cruby_bindings.inc.rs");
}

use std::sync::atomic::{AtomicUsize, Ordering};
use std::alloc::{GlobalAlloc, Layout, System};

#[global_allocator]
pub static GLOBAL_ALLOCATOR: StatsAlloc = StatsAlloc { alloc_size: AtomicUsize::new(0) };

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
