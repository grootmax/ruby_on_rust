//! Lifetime token for Ruby VM scope enforcement.

use core::marker::PhantomData;

/// Lifetime token representing an active Ruby VM scope duration.
///
/// Functions operating on GC handles or updating object fields require
/// a valid `&RubyVm<'a>` lifetime handle to guarantee that GC values remain
/// valid for the scope duration `'a`.
#[derive(Debug)]
pub struct RubyVm<'a> {
    _marker: PhantomData<&'a ()>,
}

impl<'a> RubyVm<'a> {
    /// Creates a new `RubyVm` token for the current VM scope.
    ///
    /// # Safety
    /// Must only be created on a thread with an active Ruby VM holding the GVL.
    #[inline]
    pub unsafe fn new() -> Self {
        RubyVm {
            _marker: PhantomData,
        }
    }

    /// Executes `f` within a bounded `RubyVm` lifetime scope.
    pub fn scope<F, R>(f: F) -> R
    where
        F: for<'b> FnOnce(&'b RubyVm<'b>) -> R,
    {
        // SAFETY: VM is active for the scope of the closure.
        let vm = unsafe { RubyVm::new() };
        f(&vm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ruby_vm_scope() {
        let result = RubyVm::scope(|_vm| 42);
        assert_eq!(result, 42);
    }
}
