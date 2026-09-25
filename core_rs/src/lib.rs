//! Core Rust subsystem for Ruby on Rust

#[unsafe(no_mangle)]
pub extern "C" fn rb_core_rs_init() {
    // Core Rust initialization entrypoint
}

#[unsafe(no_mangle)]
pub extern "C" fn rb_core_rs_version() -> *const std::ffi::c_char {
    c"0.0.0".as_ptr()
}
