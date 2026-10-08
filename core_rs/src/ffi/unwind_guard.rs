//! Manual clean-up guard pattern with explicit unwind pre-conditions for C FFI calls.
//!
//! Functions in CRuby such as `rb_raise()` execute `longjmp` stack unwinding, which
//! bypasses Rust stack unwinding and skips `Drop` destructors for live Rust frames.
//!
//! To avoid the runtime setjmp overhead of `rb_protect()` while ensuring memory safety
//! and preventing resource leaks, developers using raw C API calls must explicitly flush
//! or drop owned Rust resources before invoking raising C API functions.
//!
//! [`UnwindGuard`] provides an explicit mechanism to enforce and execute resource
//! flushing prior to C longjmp calls.

use core::mem::needs_drop;

/// An explicit guard for manually flushing owned resources before calling C functions
/// that perform `longjmp` stack unwinding (such as `rb_raise()`).
///
/// Because `longjmp` bypasses Rust frame destructors (`Drop`), any owned Rust resources
/// must be explicitly flushed or dropped before the C API call is made.
///
/// # Example
/// ```ignore
/// let mut guard = UnwindGuard::with_cleanup(|| {
///     // Flush buffer or release resources
/// });
///
/// if invalid_input {
///     // Explicitly flush before calling longjmp C function
///     guard.flush();
///     unsafe { rb_raise(rb_eArgError, c"invalid input".as_ptr()); }
/// }
///
/// // On normal exit path without longjmp, dismiss guard
/// guard.dismiss();
/// ```
pub struct UnwindGuard<F: FnOnce() = fn()> {
    cleanup: Option<F>,
    flushed: bool,
}

impl UnwindGuard<fn()> {
    /// Creates a new `UnwindGuard` without a cleanup callback.
    ///
    /// Use [`flush`](UnwindGuard::flush) to explicitly record that manual resource
    /// cleanup has been completed before calling a raising C API function.
    pub const fn new() -> Self {
        Self {
            cleanup: None,
            flushed: false,
        }
    }
}

impl<F: FnOnce()> UnwindGuard<F> {
    /// Creates a new `UnwindGuard` with a specified cleanup closure.
    pub fn with_cleanup(cleanup: F) -> Self {
        Self {
            cleanup: Some(cleanup),
            flushed: false,
        }
    }

    /// Explicitly flushes the guard, executing the cleanup callback (if present)
    /// and marking the guard as flushed.
    ///
    /// Call this immediately before invoking a C function that performs `longjmp`.
    pub fn flush(&mut self) {
        if !self.flushed {
            self.flushed = true;
            if let Some(f) = self.cleanup.take() {
                f();
            }
        }
    }

    /// Returns `true` if this guard has been flushed, `false` otherwise.
    pub fn is_flushed(&self) -> bool {
        self.flushed
    }

    /// Dismisses the guard on normal execution paths where no `longjmp` occurred,
    /// marking it as flushed without executing the cleanup callback.
    pub fn dismiss(&mut self) {
        self.flushed = true;
        self.cleanup = None;
    }
}

impl Default for UnwindGuard<fn()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: FnOnce()> Drop for UnwindGuard<F> {
    fn drop(&mut self) {
        if !self.flushed {
            if let Some(f) = self.cleanup.take() {
                f();
            }
        }
    }
}

/// Asserts that type `T` has a trivial drop implementation (`!needs_drop::<T>()`).
///
/// Calling raw C functions that execute `longjmp` bypasses Rust destructors.
/// Any local variable live across the call boundary must have a trivial drop implementation
/// or be manually dropped prior to the C call.
///
/// # Panics
/// Panics if `T` has a non-trivial `Drop` implementation.
pub fn assert_trivial_drop<T>() {
    assert!(
        !needs_drop::<T>(),
        "type has a non-trivial Drop implementation and cannot safely cross longjmp C FFI boundary"
    );
}

/// Helper function verifying that the type of `_val` has a trivial drop implementation.
pub fn ensure_trivial_drop<T>(_val: &T) {
    assert_trivial_drop::<T>();
}

/// Returns `true` if type `T` has a trivial drop implementation (i.e. does not require destructor execution).
pub const fn is_trivial_drop<T>() -> bool {
    !needs_drop::<T>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

    #[test]
    fn test_unwind_guard_basic_flush() {
        let mut guard = UnwindGuard::new();
        assert!(!guard.is_flushed());
        guard.flush();
        assert!(guard.is_flushed());
    }

    #[test]
    fn test_unwind_guard_with_cleanup_flush() {
        let flushed_flag = Cell::new(false);
        {
            let mut guard = UnwindGuard::with_cleanup(|| {
                flushed_flag.set(true);
            });
            assert!(!guard.is_flushed());
            assert!(!flushed_flag.get());

            guard.flush();
            assert!(guard.is_flushed());
            assert!(flushed_flag.get());
        }
    }

    #[test]
    fn test_unwind_guard_dismiss() {
        let cleaned_up = Cell::new(false);
        {
            let mut guard = UnwindGuard::with_cleanup(|| {
                cleaned_up.set(true);
            });
            guard.dismiss();
            assert!(guard.is_flushed());
        }
        assert!(!cleaned_up.get());
    }

    #[test]
    fn test_unwind_guard_drop_unflushed() {
        let cleaned_up = Cell::new(false);
        {
            let _guard = UnwindGuard::with_cleanup(|| {
                cleaned_up.set(true);
            });
        }
        assert!(cleaned_up.get());
    }

    #[test]
    fn test_trivial_drop_checks() {
        assert!(is_trivial_drop::<i32>());
        assert!(is_trivial_drop::<usize>());
        assert!(is_trivial_drop::<*const u8>());
        assert_trivial_drop::<u64>();

        struct NonTrivialDrop;
        impl Drop for NonTrivialDrop {
            fn drop(&mut self) {}
        }

        assert!(!is_trivial_drop::<NonTrivialDrop>());
    }

    #[test]
    #[should_panic(expected = "type has a non-trivial Drop implementation")]
    fn test_assert_trivial_drop_panics_on_nontrivial() {
        struct CustomDrop;
        impl Drop for CustomDrop {
            fn drop(&mut self) {}
        }
        assert_trivial_drop::<CustomDrop>();
    }
}
