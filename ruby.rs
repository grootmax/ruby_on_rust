use std::ffi::{c_char, CStr};

#[cfg(feature = "yjit")]
pub use yjit::*;
#[cfg(feature = "zjit")]
pub use zjit::*;

#[allow(non_camel_case_types)]
type VALUE = usize;

#[cfg(not(test))]
unsafe extern "C" {
    fn ruby_push_include(path: *const c_char, filter: unsafe extern "C" fn(VALUE) -> VALUE);
    fn ruby_locale_path(path: VALUE) -> VALUE;
}

#[cfg(test)]
unsafe fn ruby_push_include(_path: *const c_char, _filter: unsafe extern "C" fn(VALUE) -> VALUE) {}

#[cfg(test)]
unsafe extern "C" fn ruby_locale_path(path: VALUE) -> VALUE {
    path
}

/// Appends path entries into the load path array ($LOAD_PATH).
///
/// # Safety
/// `path` must be a valid pointer to a NUL-terminated C string, or null.
#[unsafe(no_mangle)]
pub extern "C" fn ruby_incpush(path: *const c_char) {
    if path.is_null() {
        return;
    }

    // Safely convert path to CStr across FFI boundary
    let c_str = unsafe { CStr::from_ptr(path) };

    unsafe {
        ruby_push_include(c_str.as_ptr(), ruby_locale_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_ruby_incpush_null_pointer() {
        // Null pointer input must be safely handled without panicking or segfaulting
        ruby_incpush(std::ptr::null());
    }

    #[test]
    fn test_ruby_incpush_valid_path() {
        let path = CString::new("/usr/local/lib/ruby").unwrap();
        ruby_incpush(path.as_ptr());
    }
}

