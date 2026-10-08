//! Exception boundary execution core combining `rb_protect` and panic protection.

use super::protect::{jump_tag, protect};
use super::value::{Qnil, VALUE};

#[cfg(not(test))]
unsafe extern "C" {
    pub fn rb_raise(exc: VALUE, fmt: *const core::ffi::c_char, ...) -> !;
    pub static rb_eRuntimeError: VALUE;
}

#[cfg(test)]
mod mock_raise {
    use super::*;
    use core::ffi::c_char;

    #[allow(non_upper_case_globals)]
    pub static rb_eRuntimeError: VALUE = 1;

    pub unsafe fn rb_raise(_exc: VALUE, _fmt: *const c_char) -> ! {
        panic!("rb_raise called: RuntimeError");
    }
}

#[cfg(test)]
use mock_raise::*;

/// Executes `f` within a safe FFI exception boundary.
///
/// Automatically handles Ruby exceptions (via `rb_protect` and `jump_tag`) and
/// catches Rust panics, converting unhandled panics safely into Ruby errors
/// without process termination.
///
/// # Safety
/// Must be called on a thread with active Ruby VM holding the GVL.
pub unsafe fn execute_boundary<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let mut result_slot: Option<R> = None;
    let mut panic_occurred = false;

    struct ClosureRunner<'a, F, R> {
        f: Option<F>,
        result_slot: &'a mut Option<R>,
        #[allow(dead_code)]
        panic_occurred: &'a mut bool,
    }

    let mut runner = ClosureRunner {
        f: Some(f),
        result_slot: &mut result_slot,
        panic_occurred: &mut panic_occurred,
    };

    let runner_ptr = &mut runner as *mut ClosureRunner<'_, F, R>;

    let protect_res = unsafe {
        protect(move || {
            let runner_ref = &mut *runner_ptr;
            if let Some(f_inner) = runner_ref.f.take() {
                #[cfg(any(test, feature = "std"))]
                {
                    let panic_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f_inner()));
                    match panic_res {
                        Ok(res) => {
                            *runner_ref.result_slot = Some(res);
                        }
                        Err(_) => {
                            *runner_ref.panic_occurred = true;
                        }
                    }
                }

                #[cfg(not(any(test, feature = "std")))]
                {
                    *runner_ref.result_slot = Some(f_inner());
                }
            }
            Qnil
        })
    };

    if panic_occurred {
        unsafe {
            rb_raise(
                rb_eRuntimeError,
                c"Rust panic inside exception boundary".as_ptr(),
            );
        }
    }

    match protect_res {
        Ok(_) => {
            if let Some(val) = result_slot {
                val
            } else {
                unsafe { core::hint::unreachable_unchecked() }
            }
        }
        Err(state) => unsafe {
            jump_tag(state);
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_boundary_normal_return() {
        let res = unsafe { execute_boundary(|| 1234) };
        assert_eq!(res, 1234);
    }

    #[test]
    #[should_panic(expected = "rb_raise called: RuntimeError")]
    fn test_execute_boundary_catches_panic() {
        unsafe {
            execute_boundary::<_, ()>(|| {
                panic!("intentional panic for testing boundary");
            });
        }
    }
}
