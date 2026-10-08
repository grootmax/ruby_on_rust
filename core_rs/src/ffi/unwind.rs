//! Pre-unwind resource flushing and manual clean-up guards.
//!
//! Ruby exception raises (`rb_raise()`) and other non-local exits execute `longjmp()`.
//! `longjmp()` discards Rust stack frames without running destructors (`Drop::drop`).
//!
//! To call raw raising C APIs without the runtime overhead of `rb_protect()`,
//! developers must manually ensure that all owned resources with destructors are freed
//! or flushed before invoking the raising C function.
//!
//! [`UnwindGuard`] enforces this requirement at runtime by panicking if dropped
//! without an explicit call to [`UnwindGuard::flush`].

use core::mem::needs_drop;

/// Helper to assert that a type `T` has a trivial destructor (`!needs_drop::<T>()`).
///
/// Types held across raw FFI boundaries when calling functions that can `longjmp`
/// must implement trivial drop (or be explicitly dropped/flushed before the call).
#[inline]
pub fn assert_trivial_drop<T>() {
    assert!(
        !needs_drop::<T>(),
        "Type with non-trivial Drop held across raw FFI raising call boundary"
    );
}

/// An explicit clean-up guard that must be flushed before invoking raw C APIs
/// that execute `longjmp` (such as `rb_raise()`).
///
/// If an `UnwindGuard` is dropped without being explicitly flushed via [`flush`],
/// it panics to catch forgotten object drops or unexpected unwinding paths during
/// development and testing.
///
/// # Example
/// ```ignore
/// let mut guard = UnwindGuard::new();
/// if invalid_input {
///     // Free or flush any owned resources first...
///     guard.flush();
///     unsafe { rb_raise(rb_eArgError, c"invalid input".as_ptr()) };
/// }
/// ```
#[derive(Debug)]
pub struct UnwindGuard<T = ()> {
    resource: Option<T>,
    flushed: bool,
}

impl UnwindGuard<()> {
    /// Creates a new `UnwindGuard` with no inner resource payload.
    #[inline]
    pub fn new() -> Self {
        Self {
            resource: Some(()),
            flushed: false,
        }
    }
}

impl<T> UnwindGuard<T> {
    /// Creates a new `UnwindGuard` wrapping an owned resource `T`.
    #[inline]
    pub fn with_resource(resource: T) -> Self {
        Self {
            resource: Some(resource),
            flushed: false,
        }
    }

    /// Explicitly flushes and disarms the guard before calling a C API function
    /// that can `longjmp`. Returns the inner resource (if any).
    #[inline]
    pub fn flush(&mut self) -> Option<T> {
        self.flushed = true;
        self.resource.take()
    }

    /// Disarms the guard without taking or returning the inner resource.
    #[inline]
    pub fn dismiss(&mut self) {
        self.flushed = true;
    }

    /// Returns `true` if the guard has been explicitly flushed or dismissed.
    #[inline]
    pub fn is_flushed(&self) -> bool {
        self.flushed
    }
}

impl Default for UnwindGuard<()> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for UnwindGuard<T> {
    fn drop(&mut self) {
        if !self.flushed {
            #[cfg(test)]
            if std::thread::panicking() {
                return;
            }
            panic!("UnwindGuard dropped without explicit flush() before potential longjmp");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::value::VALUE;

    struct NonTrivialDropResource {
        _data: u32,
    }

    impl Drop for NonTrivialDropResource {
        fn drop(&mut self) {
            // Non-trivial destructor
        }
    }

    #[test]
    fn test_unwind_guard_successful_flush() {
        let mut guard = UnwindGuard::new();
        assert!(!guard.is_flushed());
        guard.flush();
        assert!(guard.is_flushed());
    }

    #[test]
    fn test_unwind_guard_resource_payload() {
        let mut guard = UnwindGuard::with_resource(42);
        assert!(!guard.is_flushed());
        let res = guard.flush();
        assert_eq!(res, Some(42));
        assert!(guard.is_flushed());
    }

    #[test]
    fn test_unwind_guard_dismiss() {
        let mut guard = UnwindGuard::new();
        guard.dismiss();
        assert!(guard.is_flushed());
    }

    #[test]
    #[should_panic(expected = "UnwindGuard dropped without explicit flush()")]
    fn test_unwind_guard_unflushed_drop_panics() {
        let _guard = UnwindGuard::new();
        // Dropped without flush()
    }

    #[test]
    fn test_assert_trivial_drop_primitives() {
        assert_trivial_drop::<i32>();
        assert_trivial_drop::<VALUE>();
        assert_trivial_drop::<*const u8>();
    }

    #[test]
    #[should_panic(expected = "Type with non-trivial Drop held across raw FFI raising call boundary")]
    fn test_assert_trivial_drop_panics_on_non_trivial() {
        assert_trivial_drop::<NonTrivialDropResource>();
    }

    #[test]
    fn test_user_scenario_explicit_pre_unwind_flushing() {
        let freed;
        {
            let mut guard = UnwindGuard::new();
            // Perform manual cleanup before invoking raw C API
            freed = true;
            guard.flush();
        }
        assert!(freed);
    }
}
