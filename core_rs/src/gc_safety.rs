//! RAII GC slot mutator wrappers and interior pointer guard handles.
//!
//! Provides zero-cost type abstractions for safe garbage collection interactions in `core_rs`:
//! - [`GcSlot<T>`]: Encapsulates object slot writes and enforces GC write barriers (`rb_gc_writebarrier`).
//! - [`GcGuard<'a, T>`]: Lifetime-bound RAII guard for interior raw pointers, executing a compiler memory barrier on drop.
//! - [`unprotect`]: Helper matching `RB_OBJ_WB_UNPROTECT` for unprotecting objects from write barriers.

use crate::ffi::api::{rb_gc_writebarrier, rb_gc_writebarrier_unprotect};
use crate::ffi::value::{SPECIAL_CONST_P, VALUE};
use core::marker::PhantomData;

/// A slot in a parent Ruby object that holds a reference to another object (`VALUE`).
///
/// Encapsulates field mutations to enforce generational GC write barriers (`rb_gc_writebarrier`).
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct GcSlot<T = VALUE> {
    ptr: *mut T,
}

impl<T> GcSlot<T> {
    /// Creates a new `GcSlot` wrapping a raw pointer to a slot.
    #[inline]
    pub const fn new(ptr: *mut T) -> Self {
        Self { ptr }
    }

    /// Returns the raw pointer to the slot.
    #[inline]
    pub const fn as_ptr(&self) -> *mut T {
        self.ptr
    }
}

impl GcSlot<VALUE> {
    /// Writes `new_val` to `slot_ptr` inside `parent` object, calling `rb_gc_writebarrier`
    /// if `new_val` is a heap object (non-special constant).
    ///
    /// Returns `parent`.
    ///
    /// # Safety
    /// `slot_ptr` must be a valid, writable pointer to a `VALUE` field inside `parent`.
    #[inline]
    pub unsafe fn write(parent: VALUE, slot_ptr: *mut VALUE, new_val: VALUE) -> VALUE {
        unsafe {
            *slot_ptr = new_val;
        }
        Self::written(parent, VALUE::MAX, new_val)
    }

    /// Executed after writing `new_val` to a field in `parent`.
    ///
    /// Invokes `rb_gc_writebarrier(parent, new_val)` if `new_val` is not a special constant.
    /// Useful for bulk or unaligned write barrier declarations.
    ///
    /// Returns `parent`.
    #[inline]
    pub fn written(parent: VALUE, _old_val: VALUE, new_val: VALUE) -> VALUE {
        if !SPECIAL_CONST_P(new_val) {
            unsafe {
                rb_gc_writebarrier(parent, new_val);
            }
        }
        parent
    }

    /// Writes `new_val` into this slot inside `parent`, triggering write barrier if needed.
    ///
    /// Returns `parent`.
    ///
    /// # Safety
    /// The pointer wrapped by `self` must be valid for write inside `parent`.
    #[inline]
    pub unsafe fn write_slot(&self, parent: VALUE, new_val: VALUE) -> VALUE {
        unsafe { Self::write(parent, self.ptr, new_val) }
    }

    /// Reads the `VALUE` from this slot.
    ///
    /// # Safety
    /// The pointer wrapped by `self` must be valid for read.
    #[inline]
    pub unsafe fn read(&self) -> VALUE {
        unsafe { *self.ptr }
    }
}

/// Unprotects an object from generational GC barriers (`RB_OBJ_WB_UNPROTECT`).
///
/// Calls `rb_gc_writebarrier_unprotect(obj)` and returns `obj`.
#[inline]
pub fn unprotect(obj: VALUE) -> VALUE {
    unsafe {
        rb_gc_writebarrier_unprotect(obj);
    }
    obj
}

/// An RAII guard that binds the lifetime of an interior raw pointer to its owner Ruby object `VALUE`.
///
/// Ensures the owner object stays alive on the stack/registers until the guard goes out of scope,
/// executing a compiler memory barrier (`core::hint::black_box`) upon drop to prevent premature GC sweeping.
#[derive(Debug)]
pub struct GcGuard<'a, T: ?Sized> {
    parent: VALUE,
    ptr: *const T,
    _marker: PhantomData<&'a T>,
}

impl<'a, T: ?Sized> GcGuard<'a, T> {
    /// Creates a new `GcGuard` keeping `parent` guarded for the lifetime `'a` of `ptr`.
    #[inline]
    pub fn new(parent: VALUE, ptr: *const T) -> Self {
        Self {
            parent,
            ptr,
            _marker: PhantomData,
        }
    }

    /// Creates a new `GcGuard` keeping `parent` guarded for the lifetime `'a` of a mutable pointer `ptr`.
    #[inline]
    pub fn new_mut(parent: VALUE, ptr: *mut T) -> Self {
        Self {
            parent,
            ptr: ptr as *const T,
            _marker: PhantomData,
        }
    }

    /// Returns the guarded parent object `VALUE`.
    #[inline]
    pub const fn parent(&self) -> VALUE {
        self.parent
    }

    /// Returns the interior pointer as a const pointer `*const T`.
    #[inline]
    pub const fn as_ptr(&self) -> *const T {
        self.ptr
    }

    /// Returns the interior pointer as a mutable pointer `*mut T`.
    #[inline]
    pub const fn as_mut_ptr(&self) -> *mut T {
        self.ptr as *mut T
    }
}

impl<'a, T> GcGuard<'a, T> {
    /// Returns a slice of `len` elements starting at the interior pointer.
    ///
    /// # Safety
    /// `self.as_ptr()` must be valid for reading `len` elements of `T`.
    #[inline]
    pub unsafe fn as_slice(&self, len: usize) -> &'a [T] {
        unsafe { core::slice::from_raw_parts(self.ptr, len) }
    }

    /// Returns a mutable slice of `len` elements starting at the interior pointer.
    ///
    /// # Safety
    /// `self.as_mut_ptr()` must be valid for reading and writing `len` elements of `T`.
    #[inline]
    pub unsafe fn as_mut_slice(&self, len: usize) -> &'a mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr(), len) }
    }
}

impl<'a, T: ?Sized> core::ops::Deref for GcGuard<'a, T> {
    type Target = *const T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'a, T: ?Sized> Drop for GcGuard<'a, T> {
    #[inline]
    fn drop(&mut self) {
        // Compiler memory barrier guaranteeing register optimization does not
        // discard the parent object before interior pointer operations complete.
        core::hint::black_box(self.parent);
        core::hint::black_box(self.ptr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::api::mock;
    use crate::ffi::value::{INT2FIX, Qnil};
    use core::sync::atomic::Ordering;

    #[test]
    fn test_gc_slot_heap_object_triggers_write_barrier() {
        mock::reset_counts();
        let parent: VALUE = 0x7f00_0000_1000;
        let mut slot_val: VALUE = Qnil;
        let heap_obj: VALUE = 0x7f00_0000_2000;

        let slot = GcSlot::new(&mut slot_val as *mut VALUE);
        assert_eq!(slot.as_ptr(), &mut slot_val as *mut VALUE);

        let res = unsafe { slot.write_slot(parent, heap_obj) };
        assert_eq!(res, parent);
        assert_eq!(slot_val, heap_obj);

        assert_eq!(mock::WRITE_BARRIER_COUNT.load(Ordering::SeqCst), 1);
        assert_eq!(mock::LAST_WB_OLD.load(Ordering::SeqCst), parent);
        assert_eq!(mock::LAST_WB_YOUNG.load(Ordering::SeqCst), heap_obj);
    }

    #[test]
    fn test_gc_slot_static_write_triggers_write_barrier() {
        mock::reset_counts();
        let parent: VALUE = 0x7f00_0000_1000;
        let mut slot_val: VALUE = Qnil;
        let heap_obj: VALUE = 0x7f00_0000_3000;

        let res = unsafe { GcSlot::write(parent, &mut slot_val, heap_obj) };
        assert_eq!(res, parent);
        assert_eq!(slot_val, heap_obj);

        assert_eq!(mock::WRITE_BARRIER_COUNT.load(Ordering::SeqCst), 1);
        assert_eq!(mock::LAST_WB_OLD.load(Ordering::SeqCst), parent);
        assert_eq!(mock::LAST_WB_YOUNG.load(Ordering::SeqCst), heap_obj);
    }

    #[test]
    fn test_gc_slot_special_const_skips_write_barrier() {
        mock::reset_counts();
        let parent: VALUE = 0x7f00_0000_1000;
        let mut slot_val: VALUE = 0x7f00_0000_2000;

        let fixnum_val = INT2FIX(42);
        let res = unsafe { GcSlot::write(parent, &mut slot_val, fixnum_val) };
        assert_eq!(res, parent);
        assert_eq!(slot_val, fixnum_val);

        assert_eq!(mock::WRITE_BARRIER_COUNT.load(Ordering::SeqCst), 0);

        let res_nil = unsafe { GcSlot::write(parent, &mut slot_val, Qnil) };
        assert_eq!(res_nil, parent);
        assert_eq!(slot_val, Qnil);

        assert_eq!(mock::WRITE_BARRIER_COUNT.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_gc_slot_written_helper() {
        mock::reset_counts();
        let parent: VALUE = 0x7f00_0000_1000;
        let old_val: VALUE = Qnil;
        let heap_obj: VALUE = 0x7f00_0000_4000;

        GcSlot::written(parent, old_val, heap_obj);
        assert_eq!(mock::WRITE_BARRIER_COUNT.load(Ordering::SeqCst), 1);
        assert_eq!(mock::LAST_WB_OLD.load(Ordering::SeqCst), parent);
        assert_eq!(mock::LAST_WB_YOUNG.load(Ordering::SeqCst), heap_obj);

        GcSlot::written(parent, heap_obj, Qnil);
        assert_eq!(mock::WRITE_BARRIER_COUNT.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_unprotect_invokes_unprotect_barrier() {
        mock::reset_counts();
        let obj: VALUE = 0x7f00_0000_5000;

        let res = unprotect(obj);
        assert_eq!(res, obj);
        assert_eq!(mock::UNPROTECT_COUNT.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_gc_guard_lifetime_and_pointer_deref() {
        let parent: VALUE = 0x7f00_0000_6000;
        let mut buffer: [u8; 4] = [10, 20, 30, 40];

        {
            let guard = GcGuard::new_mut(parent, buffer.as_mut_ptr());
            assert_eq!(guard.parent(), parent);
            assert_eq!(guard.as_ptr(), buffer.as_ptr());
            assert_eq!(guard.as_mut_ptr(), buffer.as_mut_ptr());
            assert_eq!(*guard, buffer.as_ptr());

            unsafe {
                let slice = guard.as_mut_slice(4);
                assert_eq!(slice, &[10, 20, 30, 40]);
                slice[0] = 99;
            }
        } // guard drops here, executing drop memory barrier

        assert_eq!(buffer[0], 99);
    }
}
