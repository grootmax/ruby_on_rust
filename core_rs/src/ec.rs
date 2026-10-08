//! Thread-bound execution context handle abstractions in core_rs.

use core::marker::PhantomData;
use crate::ffi::api;
use crate::ffi::VALUE;
use crate::ractor::RactorHandle;

/// Opaque C struct representing `rb_execution_context_struct`.
#[repr(C)]
pub struct rb_execution_context_struct {
    _private: [u8; 0],
}

/// Zero-cost, thread-bound handle to CRuby's thread-local execution context.
///
/// Encapsulates an opaque `*mut rb_execution_context_struct` pointer. Marked
/// `!Send` and `!Sync` via `PhantomData<&'a mut ()>` and the raw pointer so it
/// cannot leak across thread boundaries.
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct EcRef<'a> {
    ec: *mut rb_execution_context_struct,
    _marker: PhantomData<&'a mut ()>,
}

impl<'a> EcRef<'a> {
    /// Creates an `EcRef` from a raw execution context pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a `rb_execution_context_struct` or null.
    #[inline]
    pub unsafe fn from_raw(ptr: *mut rb_execution_context_struct) -> Self {
        Self {
            ec: ptr,
            _marker: PhantomData,
        }
    }

    /// Returns the underlying raw pointer.
    #[inline]
    pub fn as_ptr(&self) -> *mut rb_execution_context_struct {
        self.ec
    }

    /// Returns the current thread's thread-bound `EcRef` if present.
    #[inline]
    pub fn current() -> Option<Self> {
        let ptr = unsafe { api::rb_ec_current() };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { Self::from_raw(ptr) })
        }
    }

    /// Checks pending interrupts on this execution context.
    #[inline]
    pub fn check_interrupts(&self) {
        unsafe { api::rb_ec_check_interrupts(self.ec) }
    }

    /// Returns a handle to the current Ractor associated with this execution context.
    #[inline]
    pub fn ractor(&self) -> RactorHandle<'a> {
        let ptr = unsafe { api::rb_ec_ractor(self.ec) };
        unsafe { RactorHandle::from_raw(ptr) }
    }

    /// Returns whether the given Ruby `VALUE` is shareable across Ractor boundaries.
    #[inline]
    pub fn is_shareable(&self, val: VALUE) -> bool {
        unsafe { api::rb_ractor_shareable_p(val) }
    }

    /// Asserts that the given Ruby `VALUE` is shareable across Ractors.
    /// Raises a `RactorIsolationError` in Ruby if `val` is unshareable.
    #[inline]
    pub fn assert_shareable(&self, val: VALUE) {
        if !self.is_shareable(val) {
            unsafe { api::rb_ractor_assert_shareable(val) };
        }
    }

    /// Returns the thread ID associated with this execution context.
    #[inline]
    pub fn thread_id(&self) -> usize {
        unsafe { api::rb_ec_thread_id(self.ec) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::value::{Qnil, Qtrue};
    use core::mem::size_of;

    #[test]
    fn ec_ref_layout() {
        assert_eq!(size_of::<EcRef<'static>>(), size_of::<*mut ()>());
    }

    #[test]
    fn ec_ref_null() {
        let ec = unsafe { EcRef::from_raw(core::ptr::null_mut()) };
        assert_eq!(ec.as_ptr(), core::ptr::null_mut());
    }

    #[test]
    fn ec_current_returns_none_when_null() {
        api::mock::MOCK_EC.with(|c| c.set(core::ptr::null_mut()));
        assert!(EcRef::current().is_none());
    }

    #[test]
    fn ec_current_returns_some_when_present() {
        let dummy_ec = 0x1234 as *mut rb_execution_context_struct;
        api::mock::MOCK_EC.with(|c| c.set(dummy_ec));
        let ec = EcRef::current().unwrap();
        assert_eq!(ec.as_ptr(), dummy_ec);

        // Test thread_id
        assert_eq!(ec.thread_id(), 12345);

        // Test check_interrupts
        api::mock::INTERRUPTS_CHECKED.with(|c| c.set(false));
        ec.check_interrupts();
        assert!(api::mock::INTERRUPTS_CHECKED.with(|c| c.get()));

        // Test shareability
        assert!(ec.is_shareable(Qtrue));
        assert!(ec.is_shareable(Qnil));
        ec.assert_shareable(Qtrue);

        // Reset mock
        api::mock::MOCK_EC.with(|c| c.set(core::ptr::null_mut()));
    }
}
