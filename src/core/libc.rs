//! C runtime pieces that `no_std` code needs.

use core::ffi::c_int;

unsafe extern "C" {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn __errno_location() -> *mut c_int;
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    fn __error() -> *mut c_int;
}

#[cfg(target_os = "linux")]
pub const EINVAL: c_int = 22;
#[cfg(target_os = "linux")]
pub const ERANGE: c_int = 34;
#[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
pub const EINVAL: c_int = 22;
#[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
pub const ERANGE: c_int = 34;

/// Equivalent of `errno = value;` in C.
pub fn set_errno(value: c_int) {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    // SAFETY: __errno_location() returns this thread's errno slot.
    unsafe {
        *__errno_location() = value
    };
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    // SAFETY: __error() returns this thread's errno slot.
    unsafe {
        *__error() = value
    };
}
