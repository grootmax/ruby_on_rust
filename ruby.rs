use std::ffi::c_char;

unsafe extern "C" {
    static ruby_copyright: c_char;
    fn puts(s: *const c_char) -> std::ffi::c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> std::ffi::c_int;
}

/// Re-implementation of `ruby_show_copyright` in Rust.
/// Displays copyright information to stdout and flushes the stream synchronously.
#[unsafe(no_mangle)]
pub extern "C" fn ruby_show_copyright() {
    unsafe {
        puts(&raw const ruby_copyright);
        fflush(std::ptr::null_mut());
    }
}

#[cfg(feature = "yjit")]
pub use yjit::*;
#[cfg(feature = "zjit")]
pub use zjit::*;
