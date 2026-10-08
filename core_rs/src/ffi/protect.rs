//! `rb_protect()` and `rb_jump_tag()` for Rust callers, as well as C trampoline
//! shims for deferred exception handling across FFI boundaries.
//!
//! # Standard `rb_protect()` idiom
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
//! let ret = unsafe { protect(|| /* raising calls */ Qnil) };
//! /* cleanup */
//! let ret = match ret { Ok(v) => v, Err(state) => unsafe { jump_tag(state) } };
//! ```
//!
//! # C Trampoline Shims for Deferred Exception Handling
//!
//! To avoid closure indirection and `setjmp` overhead inside Rust, C calls that
//! may raise exceptions can be invoked through C trampolines (`rb_core_protect_call`).
//!
//! The C trampoline intercepts exceptions, records the exception state tag in the
//! execution context (`rb_execution_context_t`), and returns sentinel value `Qundef`.
//!
//! Rust checks for `Qundef` with non-zero deferred status and returns early, which
//! unwinds local Rust stack frames and runs destructors (`Drop`).
//!
//! When control returns from Rust to C, the C caller inspects the execution context
//! deferred exception state and executes `rb_jump_tag()` to re-raise the exception.

use super::value::{Qundef, VALUE};
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

    // internal/core_rs.h - C Trampoline Shims for Deferred Exception Handling
    pub fn rb_core_protect_call(proc: extern "C" fn(VALUE) -> VALUE, data: VALUE) -> VALUE;
    pub fn rb_core_protect_call0(proc: extern "C" fn() -> VALUE) -> VALUE;
    pub fn rb_core_protect_call2(
        proc: extern "C" fn(VALUE, VALUE) -> VALUE,
        arg1: VALUE,
        arg2: VALUE,
    ) -> VALUE;
    pub fn rb_core_set_deferred_status(state: c_int);
    pub fn rb_core_get_deferred_status() -> c_int;
    pub fn rb_core_clear_deferred_status();
    pub fn rb_core_check_and_jump_deferred();
}

#[cfg(test)]
use self::mock::{
    rb_core_check_and_jump_deferred, rb_core_clear_deferred_status, rb_core_get_deferred_status,
    rb_core_protect_call, rb_core_protect_call0, rb_core_protect_call2,
    rb_jump_tag, rb_protect,
};

/// Calls `f` under `rb_protect()`.
///
/// Returns `Ok(value)` when `f` returned normally and `Err(state)` when it
/// left by a non-local exit (exception, `throw`, `break`, ...). After an
/// `Err`, the pending exception is in `rb_errinfo()`: either re-raise it with
/// [`jump_tag`] or discard it with `rb_set_errinfo(Qnil)`, exactly as the C
/// being ported does.
///
/// # Safety
/// Must be called on a Ruby thread holding the GVL, as `rb_protect()`.
/// `f` must follow the exception-safety rule of [`super`].
pub unsafe fn protect<F>(f: F) -> Result<VALUE, c_int>
where
    F: FnOnce() -> VALUE + Copy,
{
    extern "C" fn trampoline<F: FnOnce() -> VALUE + Copy>(arg: VALUE) -> VALUE {
        // SAFETY: `arg` is the address of `f` in `protect` below, which
        // outlives the rb_protect() call that invokes this trampoline.
        let f = unsafe { *(arg as *const F) };
        f()
    }

    let mut state: c_int = 0;
    // SAFETY: the caller guarantees the GVL; `trampoline::<F>` reads `f`
    // through the pointer only while `f` is alive; `state` is a valid int.
    let ret = unsafe { rb_protect(trampoline::<F>, &f as *const F as VALUE, &mut state) };
    if state == 0 {
        Ok(ret)
    } else {
        Err(state)
    }
}

/// Invokes a C function `proc(data)` through the C trampoline `rb_core_protect_call`.
///
/// If an exception is raised inside `proc`, the C trampoline catches it,
/// records the exception tag in the execution context deferred status, and returns `Qundef`.
/// This wrapper detects `Qundef` with non-zero deferred status and returns `Err(state)`.
///
/// This allows the calling Rust frame to return early, unwinding local Rust stack
/// frames and running destructors (`Drop`), before control returns to C where
/// `rb_core_check_and_jump_deferred()` re-raises the exception.
///
/// # Safety
/// Must be called on a Ruby thread holding the GVL.
pub unsafe fn protect_call(
    proc: extern "C" fn(VALUE) -> VALUE,
    data: VALUE,
) -> Result<VALUE, c_int> {
    let ret = unsafe { rb_core_protect_call(proc, data) };
    if ret == Qundef {
        let status = unsafe { rb_core_get_deferred_status() };
        if status != 0 {
            return Err(status);
        }
    }
    Ok(ret)
}

/// Zero-argument variant of [`protect_call`].
///
/// # Safety
/// Must be called on a Ruby thread holding the GVL.
pub unsafe fn protect_call0(proc: extern "C" fn() -> VALUE) -> Result<VALUE, c_int> {
    let ret = unsafe { rb_core_protect_call0(proc) };
    if ret == Qundef {
        let status = unsafe { rb_core_get_deferred_status() };
        if status != 0 {
            return Err(status);
        }
    }
    Ok(ret)
}

/// Two-argument variant of [`protect_call`].
///
/// # Safety
/// Must be called on a Ruby thread holding the GVL.
pub unsafe fn protect_call2(
    proc: extern "C" fn(VALUE, VALUE) -> VALUE,
    arg1: VALUE,
    arg2: VALUE,
) -> Result<VALUE, c_int> {
    let ret = unsafe { rb_core_protect_call2(proc, arg1, arg2) };
    if ret == Qundef {
        let status = unsafe { rb_core_get_deferred_status() };
        if status != 0 {
            return Err(status);
        }
    }
    Ok(ret)
}

/// Inspects and gets the current execution context's deferred exception status flag.
pub fn get_deferred_status() -> c_int {
    unsafe { rb_core_get_deferred_status() }
}

/// Clears the deferred exception status flag.
pub fn clear_deferred_status() {
    unsafe { rb_core_clear_deferred_status() }
}

/// Re-raises the non-local exit recorded by [`protect`] (`rb_jump_tag()`).
///
/// # Safety
/// `state` must be an `Err` value returned by [`protect`] on this thread,
/// and no Rust frame up to the C caller may hold a `Drop` value.
pub unsafe fn jump_tag(state: c_int) -> ! {
    // SAFETY: forwarded from the caller.
    unsafe { rb_jump_tag(state) }
}

/// Re-raises any deferred exception recorded in the execution context.
///
/// # Safety
/// Must be called from C/FFI when control has returned from Rust.
pub unsafe fn check_and_jump_deferred() {
    unsafe { rb_core_check_and_jump_deferred() }
}

/// Stand-ins for the C functions for `make core-rs-test` (which links
/// no libruby). They model the observable contract of C trampolines and
/// deferred exception flags.
#[cfg(test)]
mod mock {
    use super::super::value::{Qnil, Qundef, VALUE};
    use core::ffi::c_int;
    use std::cell::Cell;

    std::thread_local! {
        pub static RAISE_TAG: Cell<c_int> = const { Cell::new(0) };
        pub static DEFERRED_STATUS: Cell<c_int> = const { Cell::new(0) };
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
        if tag == 0 {
            ret
        } else {
            Qnil
        }
    }

    pub unsafe fn rb_jump_tag(state: c_int) -> ! {
        panic!("rb_jump_tag({state})");
    }

    pub unsafe fn rb_core_set_deferred_status(state: c_int) {
        DEFERRED_STATUS.with(|s| s.set(state));
    }

    pub unsafe fn rb_core_get_deferred_status() -> c_int {
        DEFERRED_STATUS.with(|s| s.get())
    }

    pub unsafe fn rb_core_clear_deferred_status() {
        DEFERRED_STATUS.with(|s| s.set(0));
    }

    pub unsafe fn rb_core_check_and_jump_deferred() {
        let state = unsafe { rb_core_get_deferred_status() };
        if state != 0 {
            unsafe { rb_core_clear_deferred_status() };
            panic!("rb_jump_tag({state})");
        }
    }

    pub unsafe fn rb_core_protect_call(
        proc: extern "C" fn(VALUE) -> VALUE,
        data: VALUE,
    ) -> VALUE {
        CALLS.with(|c| c.set(c.get() + 1));
        let tag = RAISE_TAG.with(|t| t.replace(0));
        if tag != 0 {
            unsafe { rb_core_set_deferred_status(tag) };
            Qundef
        } else {
            proc(data)
        }
    }

    pub unsafe fn rb_core_protect_call0(proc: extern "C" fn() -> VALUE) -> VALUE {
        CALLS.with(|c| c.set(c.get() + 1));
        let tag = RAISE_TAG.with(|t| t.replace(0));
        if tag != 0 {
            unsafe { rb_core_set_deferred_status(tag) };
            Qundef
        } else {
            proc()
        }
    }

    pub unsafe fn rb_core_protect_call2(
        proc: extern "C" fn(VALUE, VALUE) -> VALUE,
        arg1: VALUE,
        arg2: VALUE,
    ) -> VALUE {
        CALLS.with(|c| c.set(c.get() + 1));
        let tag = RAISE_TAG.with(|t| t.replace(0));
        if tag != 0 {
            unsafe { rb_core_set_deferred_status(tag) };
            Qundef
        } else {
            proc(arg1, arg2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mock::{CALLS, RAISE_TAG};
    use super::*;
    use crate::ffi::value::{FIX2LONG, INT2FIX, Qnil};

    extern "C" fn dummy_c_fn(val: VALUE) -> VALUE {
        val
    }

    extern "C" fn dummy_c_fn0() -> VALUE {
        INT2FIX(42)
    }

    extern "C" fn dummy_c_fn2(a1: VALUE, a2: VALUE) -> VALUE {
        INT2FIX(FIX2LONG(a1) + FIX2LONG(a2))
    }

    struct DropTracker<'a>(&'a mut bool);
    impl Drop for DropTracker<'_> {
        fn drop(&mut self) {
            *self.0 = true;
        }
    }

    #[test]
    fn normal_return_passes_the_value_through() {
        let x = 20;
        let before = CALLS.with(|c| c.get());
        let r = unsafe { protect(|| INT2FIX(x + 1)) };
        assert_eq!(r, Ok(INT2FIX(21)));
        assert_eq!(CALLS.with(|c| c.get()), before + 1);
    }

    #[test]
    fn closure_reads_captured_references() {
        let buf = [1u8, 2, 3, 4];
        let r = unsafe { protect(|| INT2FIX(buf.iter().map(|&b| b as i64).sum())) };
        assert_eq!(r, Ok(INT2FIX(10)));
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
        assert_eq!(unsafe { protect(|| Qnil) }, Ok(Qnil));
    }

    #[test]
    #[should_panic(expected = "rb_jump_tag(6)")]
    fn jump_tag_forwards_the_state() {
        unsafe { jump_tag(6) }
    }

    #[test]
    fn trampoline_normal_return() {
        clear_deferred_status();
        let res = unsafe { protect_call(dummy_c_fn, INT2FIX(99)) };
        assert_eq!(res, Ok(INT2FIX(99)));
        assert_eq!(get_deferred_status(), 0);

        let res0 = unsafe { protect_call0(dummy_c_fn0) };
        assert_eq!(res0, Ok(INT2FIX(42)));

        let res2 = unsafe { protect_call2(dummy_c_fn2, INT2FIX(10), INT2FIX(20)) };
        assert_eq!(res2, Ok(INT2FIX(30)));
    }

    #[test]
    fn trampoline_deferred_exception_unwinds_rust_stack() {
        clear_deferred_status();
        let mut dropped = false;

        let res = {
            let _tracker = DropTracker(&mut dropped);
            RAISE_TAG.with(|t| t.set(6)); // TAG_RAISE
            unsafe { protect_call(dummy_c_fn, INT2FIX(1)) }
        };

        assert_eq!(res, Err(6));
        assert!(dropped, "Rust stack destructors (Drop) must execute!");
        assert_eq!(get_deferred_status(), 6);

        // Clearing deferred status resets state
        clear_deferred_status();
        assert_eq!(get_deferred_status(), 0);
    }

    #[test]
    #[should_panic(expected = "rb_jump_tag(6)")]
    fn check_and_jump_deferred_reraises() {
        clear_deferred_status();
        RAISE_TAG.with(|t| t.set(6));
        let _ = unsafe { protect_call(dummy_c_fn, INT2FIX(1)) };
        assert_eq!(get_deferred_status(), 6);
        unsafe { check_and_jump_deferred() }
    }
}
