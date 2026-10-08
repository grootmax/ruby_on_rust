//! Ractor handle abstractions in core_rs.

use core::marker::PhantomData;
use crate::ffi::api;
use crate::ffi::VALUE;

/// Opaque C struct representing `rb_ractor_struct`.
#[repr(C)]
pub struct rb_ractor_struct {
    _private: [u8; 0],
}

/// Zero-cost, thread-bound handle to a CRuby Ractor instance.
///
/// Marked `!Send` and `!Sync` via `*mut rb_ractor_struct` and `PhantomData<&'a mut ()>`.
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct RactorHandle<'a> {
    ractor: *mut rb_ractor_struct,
    _marker: PhantomData<&'a mut ()>,
}

impl<'a> RactorHandle<'a> {
    /// Creates a `RactorHandle` from a raw pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a `rb_ractor_struct` or null.
    #[inline]
    pub unsafe fn from_raw(ptr: *mut rb_ractor_struct) -> Self {
        Self {
            ractor: ptr,
            _marker: PhantomData,
        }
    }

    /// Returns the underlying raw pointer.
    #[inline]
    pub fn as_ptr(&self) -> *mut rb_ractor_struct {
        self.ractor
    }

    /// Gets the handle for the current Ractor on this thread.
    #[inline]
    pub fn current() -> Option<Self> {
        let ptr = unsafe { api::rb_current_ractor_raw_stub() };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { Self::from_raw(ptr) })
        }
    }

    /// Returns whether the given Ruby `VALUE` is shareable across Ractor boundaries.
    #[inline]
    pub fn is_shareable(&self, val: VALUE) -> bool {
        unsafe { api::rb_ractor_shareable_p(val) }
    }

    /// Asserts that the given Ruby `VALUE` is shareable across Ractor boundaries.
    /// Raises a `RactorIsolationError` in Ruby if `val` is unshareable.
    #[inline]
    pub fn assert_shareable(&self, val: VALUE) {
        if !self.is_shareable(val) {
            unsafe { api::rb_ractor_assert_shareable(val) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::value::{Qfalse, Qtrue};
    use core::mem::size_of;

    #[test]
    fn ractor_handle_layout() {
        assert_eq!(size_of::<RactorHandle<'static>>(), size_of::<*mut ()>());
    }

    #[test]
    fn ractor_handle_null() {
        let handle = unsafe { RactorHandle::from_raw(core::ptr::null_mut()) };
        assert_eq!(handle.as_ptr(), core::ptr::null_mut());
    }

    #[test]
    fn ractor_handle_current_and_shareable() {
        api::mock::MOCK_RACTOR.with(|c| c.set(core::ptr::null_mut()));
        assert!(RactorHandle::current().is_none());

        let dummy_ractor = 0x5678 as *mut rb_ractor_struct;
        api::mock::MOCK_RACTOR.with(|c| c.set(dummy_ractor));
        let handle = RactorHandle::current().unwrap();
        assert_eq!(handle.as_ptr(), dummy_ractor);

        assert!(handle.is_shareable(Qtrue));
        assert!(handle.is_shareable(Qfalse));
        handle.assert_shareable(Qtrue);

        api::mock::MOCK_RACTOR.with(|c| c.set(core::ptr::null_mut()));
    }
}
