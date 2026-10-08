//! Tests for procedural macros (ruby_exception_boundary, ruby_gc_struct, gc_write_barrier).

#[cfg(test)]
mod tests {
    use crate::ffi::value::{FIX2LONG, INT2FIX, VALUE};
    use crate::gc::GcValue;
    use crate::vm::RubyVm;
    use crate::{ruby_exception_boundary, ruby_gc_struct};

    #[ruby_exception_boundary]
    pub unsafe fn sample_boundary_fn(a: VALUE, b: VALUE) -> VALUE {
        let x = FIX2LONG(a);
        let y = FIX2LONG(b);
        INT2FIX(x + y)
    }

    #[ruby_exception_boundary]
    pub unsafe fn sample_panic_fn() -> VALUE {
        panic!("test panic inside macro boundary");
    }

    #[ruby_gc_struct]
    pub struct TestGcObject<'a> {
        pub field1: GcValue<'a>,
        pub field2: VALUE,
    }

    #[test]
    fn test_ruby_exception_boundary_macro_normal() {
        let res = unsafe { sample_boundary_fn(INT2FIX(10), INT2FIX(20)) };
        assert_eq!(res, INT2FIX(30));
    }

    #[test]
    #[should_panic(expected = "rb_raise called: RuntimeError")]
    fn test_ruby_exception_boundary_macro_panic() {
        unsafe {
            sample_panic_fn();
        }
    }

    #[test]
    fn test_ruby_gc_struct_macro_setters() {
        RubyVm::scope(|vm| {
            let mut obj = TestGcObject {
                field1: GcValue::new(vm, INT2FIX(1)),
                field2: INT2FIX(2),
            };

            // Test macro-generated write barrier setters requiring &RubyVm<'a>
            obj.set_field1(vm, INT2FIX(100));
            assert_eq!(obj.field1.read(vm), INT2FIX(100));

            obj.set_field2(vm, INT2FIX(200));
            assert_eq!(obj.field2, INT2FIX(200));
        });
    }
}
