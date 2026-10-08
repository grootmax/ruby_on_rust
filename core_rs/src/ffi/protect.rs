//! `rb_protect()` and `rb_jump_tag()` for Rust callers.
//!
//! The C idiom
//!
//! ```c
//! int state = 0;
//! VALUE ret = rb_protect(func, arg, &state);
//! /* cleanup */
//! if (state) rb_jump_tag(state);
//! ```
//!
//! is written in a port as
//!
//! ```ignore
//! let ret = unsafe { protect(|| /* raising calls */ Value::Qnil) };
//! /* cleanup */
//! let ret = match ret { Ok(v) => v, Err(state) => unsafe { jump_tag(state) } };
//! ```
//!
//! The closure must be `Copy`, so it cannot own a value with a destructor:
//! when the code inside raises, the frames of the closure and of the
//! trampoline below are discarded by `longjmp()` (see the exception-safety
//! rule in [`super`]).  The same rule applies to the closure's own body.

use super::value::{Value, VALUE};
use core::ffi::c_int;

#[cfg(not(test))]
unsafe extern "C" {
    // include/ruby/internal/intern/proc.h
    /// `VALUE rb_protect(VALUE (*func)(VALUE args), VALUE args, int *state)`
    pub fn rb_protect(func: extern "C" fn(VALUE) -> VALUE, args: VALUE, state: *mut c_int)
        -> VALUE;
    // include/ruby/internal/intern/eval.h
    /// `void rb_jump_tag(int state)`, noreturn.
    pub fn rb_jump_tag(state: c_int) -> !;
}

#[cfg(test)]
use self::mock::{rb_jump_tag, rb_protect};

/// Calls `f` under `rb_protect()`.
///
/// Returns `Ok(value)` when `f` returned normally and `Err(state)` when it
/// left by a non-local exit (exception, `throw`, `break`, ...).  After an
/// `Err`, the pending exception is in `rb_errinfo()`: either re-raise it with
/// [`jump_tag`] or discard it with `rb_set_errinfo(Qnil)`, exactly as the C
/// being ported does.
///
/// # Safety
/// Must be called on a Ruby thread holding the GVL, as `rb_protect()`.
/// `f` must follow the exception-safety rule of [`super`].
pub unsafe fn protect<F, R>(f: F) -> Result<Value, c_int>
where
    F: FnOnce() -> R + Copy,
    R: Into<Value>,
{
    extern "C" fn trampoline<F: FnOnce() -> R + Copy, R: Into<Value>>(arg: VALUE) -> VALUE {
        // SAFETY: `arg` is the address of `f` in `protect` below, which
        // outlives the rb_protect() call that invokes this trampoline.
        let f = unsafe { *(arg as *const F) };
        f().into().0
    }

    let mut state: c_int = 0;
    // SAFETY: the caller guarantees the GVL; `trampoline::<F, R>` reads `f`
    // through the pointer only while `f` is alive; `state` is a valid int.
    let ret = unsafe { rb_protect(trampoline::<F, R>, &f as *const F as VALUE, &mut state) };
    if state == 0 { Ok(Value(ret)) } else { Err(state) }
}

/// Resumes the non-local exit recorded by [`protect`] (`rb_jump_tag()`).
///
/// # Safety
/// `state` must be an `Err` value returned by [`protect`] on this thread,
/// and no Rust frame up to the C caller may hold a `Drop` value.
pub unsafe fn jump_tag(state: c_int) -> ! {
    // SAFETY: forwarded from the caller.
    unsafe { rb_jump_tag(state) }
}

/// Stand-ins for the two C functions, for `make core-rs-test` (which links
/// no libruby).  They model only the observable contract `protect` relies
/// on: `func(args)` is called once, and `*state` is 0 on a normal return or
/// the tag of a non-local exit, in which case the return value is `Qnil`.
/// A test makes the "C" side raise by setting `RAISE_TAG` before the call.
#[cfg(test)]
mod mock {
    use super::super::value::{Qnil, VALUE};
    use core::ffi::c_int;
    use std::cell::Cell;

    std::thread_local! {
        pub static RAISE_TAG: Cell<c_int> = const { Cell::new(0) };
        pub static CALLS: Cell<u32> = const { Cell::new(0) };
    }

    pub unsafe fn rb_protect(
        func: extern "C" fn(VALUE) -> VALUE,
        args: VALUE,
        state: *mut c_int,
    ) -> VALUE {
        CALLS.with(|c| c.set(c.get() + 1));
        let ret = func(args);
        let tag = RAISE_TAG.with(|t| t.replace(0));
        // SAFETY: `state` points at protect()'s local.
        unsafe { *state = tag };
        if tag == 0 { ret } else { Qnil }
    }

    pub unsafe fn rb_jump_tag(state: c_int) -> ! {
        panic!("rb_jump_tag({state})");
    }
}

#[cfg(test)]
mod tests {
    use super::mock::{CALLS, RAISE_TAG};
    use super::*;
    use crate::ffi::value::{Value, INT2FIX, Qnil};

    #[test]
    fn normal_return_passes_the_value_through() {
        let x = 20;
        let before = CALLS.with(|c| c.get());
        let r = unsafe { protect(|| INT2FIX(x + 1)) };
        assert_eq!(r, Ok(Value::int2fix(21)));
        assert_eq!(CALLS.with(|c| c.get()), before + 1);
    }

    #[test]
    fn closure_reads_captured_references() {
        let buf = [1u8, 2, 3, 4];
        let r = unsafe { protect(|| INT2FIX(buf.iter().map(|&b| b as i64).sum())) };
        assert_eq!(r, Ok(Value::int2fix(10)));
    }

    #[test]
    fn non_local_exit_returns_its_state() {
        // TAG_RAISE is 6 in vm_core.h.
        let r = unsafe {
            protect(|| {
                RAISE_TAG.with(|t| t.set(6));
                INT2FIX(1)
            })
        };
        assert_eq!(r, Err(6));
        // The next call is unaffected.
        assert_eq!(unsafe { protect(|| Qnil) }, Ok(Value::Qnil));
    }

    #[test]
    fn drops_local_rust_variables_on_exception_and_normal_return() {
        use std::cell::Cell;

        struct Guard<'a>(&'a Cell<u32>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }

        let dropped = Cell::new(0);

        // 1. Normal return case
        {
            let _guard = Guard(&dropped);
            let r = unsafe { protect(|| Value::Qnil) };
            assert_eq!(r, Ok(Value::Qnil));
            assert_eq!(dropped.get(), 0);
        }
        assert_eq!(dropped.get(), 1);

        // 2. Exception / non-local exit case
        dropped.set(0);
        let result = {
            let _guard = Guard(&dropped);
            let r = unsafe {
                protect(|| {
                    RAISE_TAG.with(|t| t.set(6));
                    Value::Qnil
                })
            };
            r
        };
        assert_eq!(result, Err(6));
        assert_eq!(dropped.get(), 1);
    }

    #[test]
    #[should_panic(expected = "rb_jump_tag(6)")]
    fn jump_tag_forwards_the_state() {
        unsafe { jump_tag(6) }
    }
}
