//! Shared code between YJIT and ZJIT.
#![warn(unsafe_op_in_unsafe_fn)] // Adopt 2024 edition default when targeting 2021 editions

pub mod cruby_alloc;

pub use cruby_alloc::{CRubyGlobalAlloc, CRubyMemoryGuard, shrink_memory_guards};

#[global_allocator]
pub static GLOBAL_ALLOCATOR: CRubyGlobalAlloc = CRubyGlobalAlloc::new();

