//! GC lifetime handles and write barrier stubs.

use crate::ffi::value::VALUE;
use crate::vm::RubyVm;
use core::marker::PhantomData;

/// A GC-managed handle parameterized by the VM lifetime `'a`.
///
/// Direct access to the underlying `VALUE` requires a valid `&RubyVm<'a>` token,
/// preventing raw `VALUE` leaks outside active VM scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GcValue<'a> {
    value: VALUE,
    _marker: PhantomData<&'a ()>,
}

impl<'a> GcValue<'a> {
    /// Creates a new `GcValue` bound to the given VM lifetime.
    pub fn new(_vm: &RubyVm<'a>, value: VALUE) -> Self {
        GcValue {
            value,
            _marker: PhantomData,
        }
    }

    /// Reads the raw `VALUE` held by this handle. Requires an active `RubyVm<'a>`.
    #[inline]
    pub fn read(&self, _vm: &RubyVm<'a>) -> VALUE {
        self.value
    }

    /// Writes a new `VALUE` into this handle, running the GC write barrier stub.
    /// Requires an active `RubyVm<'a>`.
    #[inline]
    pub fn write(&mut self, vm: &RubyVm<'a>, new_val: VALUE) {
        write_barrier(self, vm, new_val);
    }
}

/// Stand-in / stub write barrier function for GC slot field updates requiring `RubyVm<'a>` access.
///
/// In CRuby, write barriers (e.g. `RB_OBJ_WRITE`) notify the GC when an old object
/// references a new object.
#[inline]
pub fn write_barrier<'a, T: GcWriteSlot<'a>>(slot: &mut T, vm: &RubyVm<'a>, new_val: VALUE) {
    slot.write_slot(vm, new_val);
}

/// Trait for fields/slots that support write barrier updates.
pub trait GcWriteSlot<'a> {
    fn write_slot(&mut self, vm: &RubyVm<'a>, new_val: VALUE);
}

impl<'a> GcWriteSlot<'a> for GcValue<'a> {
    fn write_slot(&mut self, _vm: &RubyVm<'a>, new_val: VALUE) {
        self.value = new_val;
    }
}

impl<'a> GcWriteSlot<'a> for VALUE {
    fn write_slot(&mut self, _vm: &RubyVm<'a>, new_val: VALUE) {
        *self = new_val;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::value::INT2FIX;

    #[test]
    fn test_gc_value_read_write_requires_vm() {
        RubyVm::scope(|vm| {
            let mut gc_val = GcValue::new(vm, INT2FIX(10));
            assert_eq!(gc_val.read(vm), INT2FIX(10));

            gc_val.write(vm, INT2FIX(20));
            assert_eq!(gc_val.read(vm), INT2FIX(20));
        });
    }
}
